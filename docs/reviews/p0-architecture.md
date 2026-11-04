# Review: design against the Phase 0 facts (early run)

Card: `docs/tasks/p0-architecture-review.md`. Reviewer: `wakeshot-reviewer`, 2026-11-03, about 03:15.

Read: `CLAUDE.md`, `docs/product.md`, `docs/architecture.md`, `docs/plan.md`, `docs/phases/00-facts.md`,
`docs/research.md`, all `docs/tasks/p0-*.md` cards, `docs/research/distribution.md`,
`docs/research/claude-code-signals.md`, `docs/research/rivals-lidguard.md`, parts of
`docs/research/rivals-others.md`, and spike code where a finding needed it (`spikes/lid-sleep/src/modes.rs`,
`events.rs`, `ticks.rs`, `spikes/claude-watch/src/procs.rs`, `summary.rs`, `spikes/power-settings/src/watch.rs`).
Nothing was run except read-only file reads. The orchestrator's keep-awake process is test setup and is ignored.

Counts: 1 blocker, 7 major, 15 minor. Findings that wait on Haik's morning results say "depends on".

## Tests to add before Haik's morning steps

Haik does the physical steps once. These are facts the design depends on that the pending steps do not settle, or
settle only weakly. Commands use the `lidspike` function from `p0-spike-lid-sleep` "Before you start". No spike code
change is needed for 1 to 4; `--work`, `--max-minutes` and `--end` exist (`spikes/lid-sleep/src/modes.rs:446-459`).

### Needs Haik (card `p0-spike-lid-sleep`)

1. **Replace step D with "dock, then undock and unplug with the lid closed"** (about 5 minutes, charger in and DELL
   connected at the start).
   1. `lidspike lid --end none --no-lock`
   2. At READY close the lid. Listen for the sound. Check that the DELL stays on. Wait 30 s.
   3. Lid still closed: unplug the DELL cable. Wait 60 s.
   4. Lid still closed: unplug the charger. Wait 60 s. Plug it back in. Wait 30 s.
   5. Open the lid. Wait for DONE.

   Read: the `displays` and `AC` fields per tick, any GAP and when it started against steps 3 and 4, and the
   Kernel-Power 105, 506 and 507 lines. Settles three things no step covers: whether the internal path drops at close
   (D's own goal), what undocking with the lid closed does (finding 5 needs it to switch rules), and the "closed laptop
   goes into the bag" path, which is an AC to DC switch with the lid closed (`rivals-others.md:324-326`, trap 12, "not
   checked"). Step E then starts with the DELL already unplugged.

2. **Make step I2 a long hold** (about 22 minutes unattended, battery above 50%, DELL unplugged):
   `lidspike lid --work 1200 --max-minutes 25`. At READY close the lid, leave it closed 22 minutes, open, sign in,
   wait for DONE. Today every lid-closed hold is only 60 s (`--work` default, `modes.rs:455`), so nothing tests the
   real use: a closed laptop on battery working for many minutes. Read: the first RESULT line. A gap that starts about
   900 s after the last key press (Haik's own 600 s battery timeout plus 5 minutes) means the battery rule also stops
   a closed laptop, so Wakeshot must write the DC sleep timer for lid-closed work (`product.md:50-51`). Step C tests
   the rule only with the lid open and a 60 s copy. The run also gives 20 minutes of closed-lid heat, unbiased versus
   wall time in any gap, and the lock and sound that I2 already checks.

3. **No new step, add to the reading guide for H** (`p0-spike-lid-sleep.md:123-130`): note when the `locked` field
   turns true against the `display` off callback after resume. That answers whether Windows locks by itself at the
   screen-off standby entry while a request is held, which decides finding 6. Step C cannot answer it: the
   power-settings `hold` log has no lock state (`spikes/power-settings/src/watch.rs` logs display state, not lock).

4. **Optional, only if Haik wants a heat guard (finding 18)**: during new step D, in a second Terminal before closing
   the lid: `lidspike heat --seconds 240 --load-threads 8 --load-at 10 --load-seconds 200`. Gives a closed-lid heat
   curve under load.

5. **Skip to pay for 1 and 2** (saves about 20 minutes):
   - G and I3 (wake timers). No part of `product.md` or `architecture.md` uses a wake timer: a job further than 1 hour
     "runs once on wake" (`product.md:29-30`), and Windows already hibernates from standby by itself
     (`research.md:390-394`). Keep them only if Phase 1 names a use.
   - C7 (`hold --request stes`). The product uses `PowerSetRequest` only (`product.md:47`).

### Agent only (no Haik; run now)

6. **Liveness check from an unelevated process against an elevated `claude.exe`** (card `p0-claude-live-tests`,
   reopen with one row). All Claude live tests ran from this elevated session (`p0-spike-power-settings.md:105-107`),
   so rule 2 was never checked the way Wakeshot will run it. Add a one-shot mode to `spikes/claude-watch` that reads
   `<pid>.json`, calls `procs::verdict(pid, procStart)` and also waits on the handle for 1 s. Run it from a `.cmd`
   opened with `explorer.exe` (the method in `p0-spike-lid-sleep.md` log 02:48, not `limited`), against the pid of an
   elevated `claude.exe` (the orchestrator). Expected pass: "alive, start time matches". If it fails, test
   `NtQuerySystemInformation(SystemProcessInformation)` `CreateTime` as the fallback (no handle needed). See finding 2.

7. **Keep the live-test fixtures** (card `p0-claude-live-tests`). The watcher logs were not kept
   (`claude-code-signals.md:327-328`), and the card suggests deleting the `cctest*` folders in `~/.claude/projects`,
   which hold the only raw transcripts of the 14 tests. Before that, copy the relevant transcript lines and status file
   snapshots, with prompts and paths redacted (never a `.key` file), to `spikes/claude-watch/fixtures/`. Phase 2 and 4
   parser tests need real shapes, not hand-made ones.

## Findings

### Blocker

**1. The journal lives in the folder Velopack wipes.**
- Where: `architecture.md:48` (`%LOCALAPPDATA%\Wakeshot\`); `distribution.md:43-46`, `60`, `106-109`, `114-116`.
- What breaks: Velopack installs to `%LocalAppData%\{packId}`, so pack id `Wakeshot` is the same folder.
  `winget upgrade` runs Setup.exe over the install: it kills the tray app and the guard (both run from that folder),
  runs no uninstall or obsolete hook, and deletes the old folder (the trial's test file was gone). If an agent was
  working, the lid action stays Do nothing (and the DC timer, if written) and the journal with Haik's original values
  is gone. The next start finds no journal, so Do nothing looks like the user's own value.
  Config and logs go too. The guard cannot help here: Velopack kills it in every
  flow (`distribution.md:36-49`).
- Fix: keep data outside the pack folder (another pack id such as `WakeshotApp`, or another data folder name). Make
  the `--veloapp-install` and `--veloapp-updated` hooks run "restore if a journal exists", like app start. Keep a
  second copy of the original values outside that folder (an HKCU key), or use the self-marking plan
  copy (finding 15). Optional: winget `UpgradeBehavior: uninstallPrevious`, so the uninstall hook runs first (not
  verified with Velopack). Uninstall is safe only because its hook runs after the kill and before the folder removal
  (`distribution.md:36-39`); a failed hook does not stop it, so keep that hook small.

### Major

**2. The Claude liveness rule was never tested from an unelevated process.** Unverified; a blocker if test 6 fails.
- Where: `claude-code-signals.md:226` (rule 2); `spikes/claude-watch/src/procs.rs:19-25` (an `OpenProcess` failure
  becomes "dead or no access"); `p0-spike-power-settings.md:105-107` (this session is High integrity).
- What breaks: Wakeshot runs at Medium. Haik's Claude Code runs elevated. If a Medium process cannot open a High
  `claude.exe` with `PROCESS_QUERY_LIMITED_INFORMATION | SYNCHRONIZE`, rule 2 counts every elevated session as dead
  and Wakeshot never holds for Haik's own work. Same for an elevated `wakeshot run`. Whether the open works depends on
  the elevated process's DACL, not only on integrity rules: not known.
- Fix: run test 6 now. If it fails, read creation times from the system process list and poll for exit.

**3. A hung tray app holds the PC awake with no limit.**
- Where: `architecture.md:33` (one thread), `45-46` (the guard reacts only to process death); limits in
  `product.md:56-58`.
- What breaks: lid closed, laptop in a bag, agent working. The one thread blocks (a deadlock, a stuck WinRT call, a
  slow file read). The request stays held because the process lives. The time limit, the battery floor and any heat
  check run on that same thread, so none fires; the guard sees a live process. The laptop runs until Windows' critical
  battery action, or forever on AC.
- Fix: the guard checks the tray window every 30 s with `SendMessageTimeout(WM_NULL)` and `SMTO_ABORTIFHUNG`; after
  about 2 minutes with no answer it ends the tray process (Windows then drops the requests) and restores. Run the guard
  whenever a hold is active, not only while settings are overridden. Keep blocking calls off the loop thread.

**4. No fallback when hibernate is off or fails.** Depends on: `p0-spike-lid-sleep` steps F2 and E.
- Where: `product.md:56-58`; `research.md:409-410`.
- What breaks: many users turn hibernate off (`powercfg /h off`). At the lid-closed time limit or the battery floor
  the design says only "hibernate". If that is not possible, the request stays held and the closed laptop keeps
  running.
- Fix: if `IsPwrHibernateAllowed` is false or `SetSuspendState(TRUE)` fails: release all requests, restore settings,
  notify. A closed lid with no request should then reach standby (E's second RESULT line will show it). Re-check the
  lid right before acting.

**5. Docked use is defined only by the user's own lid action.** Depends on: `p0-spike-lid-sleep` step D (and new D
from test 1).
- Where: `product.md:53-54`, `58-59`; `00-facts.md:46`; `p0-spike-lid-sleep.md:221-222`; LidGuard fixes `f3bac30`,
  `9c5f2c2` (`rivals-lidguard.md` 5).
- What breaks: Haik's own setup is lid action Sleep plus the DELL. While an agent works, Wakeshot sets Do nothing, so a
  lid close keeps the DELL on. Wakeshot then locks him at once, hibernates him at the time limit, and sleeps the PC
  "soon after" work ends, while he types on the DELL.
- Fix: docked means the user's lid action is Do nothing, or an external display is active (`QueryDisplayConfig`). When
  docked, skip lock, sound, time limit and end-of-work sleep. If the count drops to internal only while the lid is
  closed, apply the closed-lid rules then. Before any end-of-work sleep or hibernate, also require some minutes with no
  input.

**6. The idle lock rule fires in the wrong case.** Depends on: `p0-spike-lid-sleep` step H (test 3).
- Where: `product.md:52-53`; `research.md:41-42`, `365-368`, `399-402`.
- What breaks: Haik watches a video on battery while an agent works. The player holds a display request (Chromium
  "prevent-display-sleep"), so Windows would not sleep. After 10 minutes without input, Wakeshot locks the PC mid-video.
  On Modern Standby the display timeout (300 s AC, 180 s DC here) comes long before the sleep timeout, and screen off
  already starts standby (and, inferred, the lock seen at 02:44). So the rule mostly fires exactly when another app
  keeps the screen on.
- Fix: lock when the display was turned off by the idle timeout, not when idle time passes the sleep timeout. If H
  shows Windows locks at screen off by itself with a request held, drop the idle part. Keep the lid-close lock.

**7. `busy` has no time limit.**
- Where: `product.md:28` against `40-41`; `claude-code-signals.md:64-65` (no heartbeat);
  `rivals-lidguard.md` (port 2).
- What breaks: a hung `claude.exe` (a stuck MCP call, a network stall) keeps `busy` and a live pid. With the lid open
  on AC the PC never sleeps, all weekend: the one-signal-with-no-expiry shape that `product.md:40-41` rules out.
- Fix, Haik's call for the value: `busy` with no growth of the transcript, subagent transcripts or task output files
  for about 2 hours stops holding, with a notification. It must be long: a quiet foreground command also shows `busy`
  with no growth (`rivals-lidguard.md` 2, Avoid 3). Unknown status values (treated as busy by rule 3) get
  the same cap.

**8. Updates can land in the middle of work.**
- Where: `product.md:106-107`; `distribution.md:36-49`.
- What breaks: (a) an auto-update with the lid closed kills the tray and the guard, Windows drops the request, and the
  closed laptop goes to standby mid-task. (b) `wakeshot.exe run` runs from the install folder too, so it is killed:
  `wakeshot run -- build && deploy` sees a failure and skips deploy, the hold vanishes, and the child runs on unheld
  (or dies, if it sits in a kill-on-close job).
- Fix: the app applies an update only when nothing holds and nothing is overridden (the app picks the moment with
  Velopack). Record the child pid in the hold. A `winget upgrade` during a `run` still kills the wrapper: document it.

### Minor

**9. Proposed Claude rule 1 misses Desktop sessions.** `claude-code-signals.md:284-287` lists `sdk-*` and
`claude-vscode`, but the SDK path likely covers Desktop too (`:103`), whose entrypoints are `claude-desktop`,
`claude-desktop-3p` and `local-agent` (`:46`). A Desktop session with a background build shows `idle` and is never
checked, so the PC can sleep mid-build. Fix: run the transcript task check for every session whose entrypoint is not
`cli` (or for all idle sessions), and apply the Open 1 one-hour cap to those tasks too.

**10. Rule 5's line filter drops the lines that carry the fire time.** Rule 5 (`claude-code-signals.md:229`) keeps
only lines containing the tool names, but the result line with `nextFireAt`, `scheduledFor` and the job `id` carries
`tool_use_id` and `toolUseResult`, not the name (`:153`). The spike gets this right by pairing ids
(`spikes/claude-watch/src/summary.rs:245-260`); the written rule does not. Rule 1's markers (`backgroundTaskId`,
`<task-notification>`) and rule 4's `session_cron_carry` are also missing. Fix: write the rule as the spike works:
remember tool_use ids of watched tools, take the result by `tool_use_id`, skip `is_error` results. The parser keeps
state between reads and re-reads a transcript from its start after a Wakeshot restart.

**11. KISS: cut parts of the Claude timer rules.**
- Durable jobs (rule 7, change 6): off in this build. Drop for v1; add when a file is seen.
- `claude agents --json` (rule 10, change 7): a 335 MB process, open bug #101182, no `bg` use by Haik, and `bg`
  sessions write status files and transcripts too (`claude-code-signals.md:182`). Drop for v1.
- Rule 6 (jitter formula, median gap): the product rule is only "next run within 1 hour". Holding for a recurring job
  whose cron period is 1 hour or less gives almost the same answer with no jitter math. Longer periods: next cron time
  after the last fire. One-shots: the exact `nextFireAt` or `scheduledFor`.
- Change 10 (no hold for the hidden loop fallback) contradicts `product.md:29` literally. Agree with the change;
  reword product.md to "a pending timer Wakeshot can see".

**12. KISS: three mechanisms to see the user's setting changes.** `p0-spike-power-settings.md:175-177` proposes the
registry notification (undocumented key), the setting notification and a slow poll. A poll of the few values every 5
to 10 s while overriding costs under 1 ms per read (`research.md:335`).
Keep the power source and active plan notifications the core needs anyway. Add
the registry watch only if a 10 s delay proves to be a real problem.

**13. "One thread" does not match the callback notifications.** `architecture.md:33`; the spikes use
`DEVICE_NOTIFY_CALLBACK`, which runs on a system thread and needs a locked queue
(`spikes/lid-sleep/src/events.rs:27-30`, `143-157`). Fix: the tray already owns a hidden window, so register with the
window handle and handle `WM_POWERBROADCAST`, or have callbacks
only post a message to the loop. Say which in Phase 1.

**14. "Cleanup runs on drop" does not hold where it matters.** `product.md:20-21` gives drop cleanup as a reason for
Rust, but the spike release profile uses `panic = "abort"` (`research.md:283`, `spikes/tray/Cargo.toml:46`), which
skips every `Drop` on panic, and `TerminateProcess` skips it too. Fix: never put settings restore only in `Drop`. A
panic hook runs before the abort, so restore there; keep the guard and the restore at start.

**15. The lost-journal risk favors the plan copy.** Depends on: `p0-spike-power-settings` step B.
`architecture.md:59-61` picks in-place edits. With in-place edits a lost journal leaves Do nothing in the user's plan
with no marker. A copy marks itself:
its name and description (`PowerWriteFriendlyName`, `PowerWriteDescription`, the latter not tested) can carry the
original plan GUID, so restore needs no journal: switch to that GUID, delete the copy. The copy's cost is real (a
change the user makes during a hold lands on the copy, `p0-spike-power-settings.md:181-184`). Weigh both after B.

**16. Two signed-in users fight over one machine-wide plan.** `architecture.md:41` says one instance per user, but
plans live under HKLM (`research.md:328-329`). With fast user switching, B's Wakeshot can save A's Do nothing as B's
"original", or A's restore removes B's override while B's agent works. Rare here. Fix: one settings owner per machine
(a `Global\` mutex); other sessions hold requests only. Or document single user.

**17. Guard gaps (not verified).** `architecture.md:45-46`. If the guard is a child of the tray, Task Manager's "End
task" on the app group may end both: start it detached and test in Phase 3. If the user turns off start at login, the
"next start" restore never comes after a crash; a logon task would cover that.
Decide whether it is worth it.

**18. No heat guard in the product.** `product.md:56-59` has none, though the spike found a sensor that works without
admin (`research.md:428-437`, `_CRT` 113 C) and the motivating story is a hot laptop in a bag (`product.md:40-41`).
The time limit bounds how long, not how hot. Haik's call. Lean option: lid closed, not docked, zone above a fixed value
for a few minutes, then hibernate (with the finding 4 fallback). No setting.

**19. `wakeshot run` is not on PATH.** It is one of the two core sources (`product.md:8-9`), but Velopack has no PATH
feature (`distribution.md:50-51`), so users would type the full install path. Fix: the install and updated hooks add
the `current` folder to the user Path and broadcast `WM_SETTINGCHANGE`; the uninstall hook removes it. Not verified
that `current` keeps its path across updates.

**20. Hold files and CLI control have gaps.** `architecture.md:42-44`, `53`. The CLI's "keep awake" writes a hold, but
a hold whose process is gone is ignored and the CLI exits at once, so such holds need the tray's pid or an expiry, and
"ends at sign-out" then needs its own rule. `status` reads files, but no file holds the tray's state. `config.json` is
not in the watched folders (`architecture.md:34`). A public format with a FILETIME start time is hard to write from a
script: keep it internal in v1.

**21. "Restore my power settings" during a hold is not defined.** `product.md:88`. If the next tick re-applies the
override, the item does nothing; if not, the icon still says "Holding" while a lid close now sleeps. Fix: treat it as a
user change (`product.md:66-67`): restore, stop overriding until the work ends, keep the request, notify.

**22. WSL (Open 2): recommend no for v1.** `product.md:118`. A session file written inside WSL carries a Linux pid, so
rule 2 cannot check it against the Windows process table; stale files could not be dropped and would hold forever.
Reading `\\wsl$` may also start a stopped distro (not verified). Revisit with a helper inside WSL.

**23. Doc contradictions and stale lines.**
- `product.md:114`: a message from Haik asks the product owner to decide Open 1 and 2 ("answer yourself and clear
  1-2 questions below"). Neither is done: Open 1 still says "not answered yet". My input: the Open 1 pick is fine with
  finding 9; Open 2 see finding 22.
- `product.md:96`: "Windows 11 hides new tray icons; not verified" is now verified (`research.md:238-242`).
- `CLAUDE.md:17` "Treat any unknown shape as no signal" against rule 3 (`claude-code-signals.md:228`, `108-110`):
  an unknown status value means working. Suggest: unparseable file = no signal plus the "update Wakeshot" notice
  (`product.md:97`); unknown status value = busy, bounded by the finding 7 cap.
- `CLAUDE.md:20` states the ExecutionRequired fact as settled; `research.md:26-30` says "Not tested by us".
- `product.md:27` "from its status files": timers and SDK background shells need the transcripts too. Say so, with the
  privacy rule (`claude-code-signals.md:237-238`).
- `product.md:40` "Every signal has a time limit" against `wakeshot run` and "Until I turn it off", which have none by
  design. Say "every inferred signal".
- `product.md:110`: SignPath approval is unlikely before the project has users, and Windows would show "SignPath
  Foundation" as publisher (`distribution.md:205-218`). The phase log already says v1 ships unsigned
  (`00-facts.md:97`); product.md does not.
- `architecture.md:36` and `research.md:120` rely on a clock that ignores sleep, but `QueryUnbiasedInterruptTime`
  matched interrupt time across Modern Standby sessions (`research.md:459-461`). Depends on: the gaps in
  `p0-spike-lid-sleep` E to I.
- `00-facts.md:40` says "hold 2h" was rejected; `product.md:77` now has Keep awake with 2 hours. The log line is
  superseded; do not use it as an input.

## Judgments on the known inputs

### Velopack kill and wipe against the data location

Blocker, finding 1. Also finding 8 (update timing) and 19 (PATH). In every Velopack flow the guard dies with the app,
so hooks plus a journal outside the pack folder are the only safety net there.

### The ten proposed Claude rule changes (`claude-code-signals.md:284-305`)

| # | Change | Verdict |
|---|---|---|
| 1 | SDK-path shells from the transcript | Agree; widen to all non-`cli` sessions and apply the Open 1 cap (finding 9) |
| 2 | `dialog open` counts as idle | Agree |
| 3 | Floats; drop a wakeup at its fire or at `scheduledFor` plus a margin | Agree; keep the margin a few minutes, since it bounds the invisible Esc cancel |
| 4 | After `/clear`, keep jobs per pid and read `session_cron_carry` | Agree; key all job state by pid, not by `sessionId` |
| 5 | Reminders come as `CronCreate` or `ScheduleWakeup` | Agree |
| 6 | Durable reader, low priority | Drop for v1 (finding 11) |
| 7 | `claude agents --json` for `bg` kinds only | Drop for v1 (finding 11) |
| 8 | Rule 2 confirmed | Confirmed only from an elevated process (finding 2, test 6) |
| 9 | `updatedAt` is not a heartbeat | Agree |
| 10 | No hold for the hidden loop fallback | Agree; reword `product.md:29` (finding 11) |

Missing from the list: a cap for `busy` (finding 7) and the rule 5 filter fix (finding 10).

### Power and lid spike implications

- Agree: no admin needed; `PowerSetActiveScheme` after every write; never read the value from a notification payload;
  compare with what we wrote to tell writes apart; non-zero `PowerSettingAccessCheckEx` means requests only; delete a
  copy only after switching away; `QueryDisplayConfig` for docked; PDH for heat; `IsPwrHibernateAllowed` before
  hibernate; letting the screen turn off (depends on E to I showing no pause of the agent).
- Change: the registry watch (finding 12); the unbiased clock (finding 23, depends on gaps).
- No consumer: wake timers (test 5).
- `SystemExecutionState` helps only if it excludes our own request, and is not needed if finding 6 is adopted.

### Design inputs in the phase work log (`00-facts.md:34-53`)

- Re-take the request after resume, new before old: agree, and do it always; it is cheap, so it need not wait for H.
- Re-add the tray icon on `TaskbarCreated` and retry at login: agree.
- Restore menu item and failed-restore warning: agree, define the hold case (finding 21).
- WSL: finding 22. App rules by exe name: after v1, moot now.
- Time limit on every "still working" signal: not met for `busy` (finding 7).
- Docked by external display: not yet in product.md (finding 5). Heat guard: not decided (finding 18).
- Re-read the plan every 10 s while holding: that is the lean answer (finding 12).
- Never answer the agent's prompts: agree.

## For the delta review after Haik's steps

- Finding 4: F2 (hibernate from the app works here) and E (standby after release with the lid closed).
- Finding 5: D and the new D from test 1.
- Finding 6: H, the `locked` field (test 3).
- Finding 15: B (Settings page, Vantage, restart with a copy active).
- Finding 23 (clock): gaps in E to I.
- Battery timer decision (`product.md:50-51`): C and the long I2 from test 2.
- Sound at lid close (`product.md:55`): D, E, I. Lock at lid close: I. Re-take after resume: H.
