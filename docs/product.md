# Product

Status on 2026-11-03: decisions fixed, Phase 1 (detailed design and skeleton) running, no product code yet.
Personal project of Haik Asatryan (GitHub `HaikAsatryan`). Free, MIT license.

## The job

Keep the PC awake only while something needs it, then let it sleep as usual. "Something" is an AI agent at work
(Claude Code first) or a long command started with `wakeshot run`. On top: keep awake on demand, like PowerToys
Awake, so nobody needs two tools. Keeping a laptop working with the lid closed is one more feature, not the core.

The user must always understand what happens to their laptop. The tray icon shows the state. A notification or a
sound explains every action Wakeshot takes on its own. The routine start and end of a hold, with the power settings
Wakeshot writes and puts back each time, show in the icon and tooltip only (Haik, 2026-11-03).

## Decided

### Platform and stack

- Windows 11 only. Native x64 and ARM64 builds. English only.
- Rust with `windows-rs`. Reason: tiny always-on footprint, no runtime, decision core is portable. Runner-up: C# .NET 10 Native AOT. Switch when Rust reviews get too slow, or when a rich window is needed
  before `windows-reactor` (Microsoft's WinUI library for Rust, preview in 2026) is stable.
- macOS and Linux are long term, not now. The decision core has no OS code so a port adds adapters only.

### What keeps the PC awake (sources)

- **Claude Code**, from its status files in `~/.claude/sessions`, plus the session transcript for timers and for
  background tasks of non-terminal sessions. Only the listed fields are parsed; prompts are never stored or logged.
  No hooks, no edits to the user's Claude settings.
  - `busy` holds. It stays `busy` while background subagents run after a turn.
  - An idle session with a pending timer (`/loop`, cron, wakeup) that the transcript shows holds when the next run is
    within 1 hour. Further out, the PC may sleep; the job runs once on wake. No hold for Claude's hidden loop
    fallback.
  - `waiting` holds only while Remote Control is on (`bridgeSessionId`), for at most 1 hour.
  - `shell` (background command while idle), and a background task found in the transcript of a non-terminal session
    (VS Code, SDK, Desktop): holds while the session process lives and the task is open. No time cap. A dev server
    that never ends holds until it or the session ends; the tooltip says what holds.
- **Other agents** after v1 (Haik, 2026-11-03: v1 ships Claude Code only, to keep v1 small; the cards are ready and
  marked `later`): Codex CLI and Copilot CLI first (`research/other-agent-signals.md`). OpenCode next (reliable
  signal, free to test). No reliable signal yet: Gemini CLI, Cursor CLI, Aider, Amp, Kiro. Logic may be ported from
  the MIT rivals with credit. Haik has personal Codex and Copilot access for one live test each. Agents not tested
  live are marked so in the docs.
- **`wakeshot run -- <command>`**: holds while the command runs. Writes a hold file, a small documented format that
  any tool or script can also write.
- **Keep awake on demand**: see UX.
- Every signal is checked against a fact, not capped by time. A Claude session counts only while its process lives
  (a wait on the process handle), so a crashed session or a leftover file never holds. A timer counts until its due
  time plus a short margin; this also ends a wakeup cancelled with Esc, which writes nothing to disk. `busy` and
  `shell` have no cap while the process lives. A hold file counts while its process lives, so `wakeshot run` ends
  with its command. Keep awake ends at its time or at sign-out. No extra safeguard for another program's bug (Haik,
  2026-11-03). Two limits stay because they are product rules, not bug guards: Claude waiting for approval
  holds at most 1 hour (the `waiting` rule above), and the battery floor. Codex and Copilot: see "Answers to the
  track plan questions".
- Later, not v1: rules for any app ("while ffmpeg runs"). Claude Code inside WSL: its session files carry Linux pids,
  which Windows cannot check, so stale files would hold forever. Revisit with a helper inside WSL.
- No plugins and no scripting. A new built-in source is new code.

### Power behavior

- Hold `PowerRequestSystemRequired` plus `PowerRequestExecutionRequired`. Hold the display only when the user asks
  ("Keep screen on").
- Take the request again after every resume. Windows ends every request when the user sleeps the PC (Start > Sleep,
  power button), and it is not honored after resume (PowerSetRequest docs; tested with Start > Sleep on AC: without a
  re-take the PC slept at the next screen off, with one it stayed awake). The tested re-take clears the request and
  sets it again on the same handle.
- Change only the power settings a test proves necessary: the lid action and the sleep timer on battery. On battery
  Windows ends every request about 5 minutes after the sleep timeout, lid open or closed (tested, `research.md`,
  "Battery rule"). Wakeshot writes the battery sleep timer together with the lid action when a hold starts, also on
  AC, so an unplug or a short AC drop on a USB-C dock never meets that rule first. The value is Never (Haik,
  2026-11-03). Tested with Never only, lid open: on battery a held request stayed honored for 16 minutes with no
  input, past the point where Haik's 10 minute timeout had ended it. A finite value only moves the point where requests
  end (5 minutes after it, by Microsoft's text; not tested).
- **Never lower security.** When the lid closes while Wakeshot holds, Wakeshot locks the PC: Windows would have
  slept, and with the lid action Do nothing it does not lock by itself (tested). When the screen turns off for idle,
  Windows locks by itself, also while a request is held (tested with sign-in "Every Time"), so Wakeshot adds no idle
  lock. Only with Keep screen on, where the screen never turns off, Wakeshot locks when idle passes the user's own
  sleep timeout. No lock if "require sign-in after sleep" is Never; which stored value means Never is not known yet
  (Windows Hello greys the choice out on the test laptop), so until a test shows it, Wakeshot locks. There is no test
  account for it, so v1 always locks at lid close while holding (Haik, 2026-11-03). No lock on lid close in docked
  use (the user's own lid action is Do nothing). Unplugging a closed laptop that then keeps running only because of
  Wakeshot locks it too, once it has been on battery for 30 seconds (Haik, 2026-11-03: the test laptop's dock dropped
  AC for 5.3 s and 6.3 s, so 5 s locked him while he worked on the external screen).
- Sounds wherever they give a useful hint (Haik, 2026-11-03):
  - Lid closed while Wakeshot holds: a short chime says the laptop keeps running.
  - Work ended with the lid closed: a short "going to sleep" sound right before Wakeshot hibernates the laptop.
  - Battery floor reached.
  - No sound for: a failed restore (the warning notification is enough), a hold that starts or ends with the lid
    open, Keep awake running out, the lid opening, AC lost or back (except when it leaves a closed laptop running on
    Wakeshot's Do nothing: then as a lid close, with the lock), the user changing a setting Wakeshot overrides, and
    "update Wakeshot". These get a notification or the icon only.
  - Every sound plays on the default audio device, unless that device is a screen's audio (HDMI or DisplayPort, often
    with no speakers); then on the laptop speakers. Headphones stay as they are. Card `p1-lid-sound` picked this and
    Haik heard the chime at two lid closes with the lock on (`research.md`, "Lid-close sound").
- When the user signs in after a closed-lid hold, a notification says how long the PC kept running with the lid closed
  and what held it (Haik, 2026-11-03).
- **Battery floor**: the Windows energy saver threshold; 20% when it is Never or Always. No Wakeshot setting. On
  battery below the floor, Wakeshot steps back at once: it releases every request, restores settings and notifies,
  so Windows rules apply. If the lid is closed, it then hibernates the laptop, but only after the same 5 minutes
  with no input as at work end (below), so someone typing on an external screen with the lid closed is not put to
  sleep. Right before it hibernates, it checks again that the lid is closed and the PC is on battery under the floor:
  a USB-C dock can drop AC for a few seconds (seen on the test laptop). When AC comes back, the hold resumes.
- With the lid closed, Wakeshot holds for as long as the work runs: no time limit and no heat guard. Someone who sets
  the lid to Do nothing by hand and puts the laptop in a bag gets no heat check either, and the hardware's own
  protection applies the same in both cases: the CPU slows itself at its limit (not tested here), and Windows shuts
  down at the critical trip point (113 C on the test laptop, `research.md`, "Limits: Kernel-Power 125").
- When work ends with the lid closed, Wakeshot hibernates the laptop once there has been no input for 5 minutes
  (Haik, 2026-11-03), so someone typing on an external screen with the lid closed is not put to sleep. It must act
  itself: putting the lid action back does not sleep a laptop whose lid is already closed (tested), and after a
  release its apps most likely run until the user's own sleep timeout (inferred; not tested with the lid closed).
  Method: `SetSuspendState(TRUE)` after `IsPwrHibernateAllowed` and enabling SeShutdownPrivilege; on this Modern
  Standby laptop `SetSuspendState(FALSE)` hibernates too (tested). Check the lid right before the call. If hibernate
  is not allowed or the call fails: release every request, restore settings and notify; Windows then most likely
  stops the laptop at the user's sleep timeout. If the user's own lid action is Do nothing, Wakeshot changed nothing,
  so it only releases its hold.

### User settings safety

1. Before any write, save the original value (atomic file) and remember the value we wrote.
2. Restore only what is still ours. If the current value differs from what we wrote, the user or another tool changed
   it: keep theirs.
3. The user's latest change wins at once. If they change a value we override, we stop overriding it, on every power
   plan, until the current work ends, and a notification says so. "Restore my power settings" counts as a user
   change: restore now, stop overriding until the current work ends, keep the power request, notify.
4. Follow a power plan switch: restore the old plan, apply to the new one.
5. Restore on: work end, quit, sign-out, next start, uninstall, and app crash or kill (guard process).
6. If a company policy blocks the write, run with power requests only and tell the user.

The crash gap is accepted (Haik, 2026-11-03): after an OS crash, a power cut or the other cases listed in
`architecture.md` ("Gap left"), nothing restores before the next sign-in, because that would need admin, or longer
in the cases of Question 11 (`phases/01-design-skeleton.md`; the default, Haik 2026-11-03 18:09). Until then the
lid action stays Do nothing and battery sleep Never. The README says so.

### UX

- No window. Tray icon, tray menu, notifications, sounds, CLI, log file. Everything the CLI controls, the menu
  controls too (except `wakeshot run`, which wraps a command).
- CLI commands (Haik, 2026-11-03): `wakeshot run -- <command>`, `wakeshot keep-awake 30m|1h|2h|4h|on|off [--screen]`
  (`on` means Until I turn it off), `wakeshot automatic on|off`, `wakeshot status`, `wakeshot restore`,
  `wakeshot log`. `keep-awake` needs the tray running and says so if it is not. `run` always runs the command, and
  warns when the tray is not running or Automatic is off.
- **Automatic**: on or off. Watches agents and `wakeshot run`. Survives a restart. Starts at login, on by default.
- **Keep awake**: Off, 30 minutes, 1 hour, 2 hours, 4 hours, Until I turn it off. Option "Keep screen on". Works
  whether Automatic is on or off. It is one more hold, so the icon, lid rules and battery floor apply the same way.
  It ends at sign-out or restart, so a forgotten "Until I turn it off" does not survive a reboot.
- Menu sketch (wording set in Phase 5):

  ```
  Holding awake: Claude Code, 2 sessions
  ------------------------------------------
  Automatic                              [x]
  Keep awake                              >   Off | 30 minutes | 1 hour | 2 hours | 4 hours
  ------------------------------------------      Until I turn it off | Keep screen on
  Restore my power settings
  Open log
  Quit
  ```

- Tray icon states: holding, idle, off. It must look good, be crisp at every display scale, and be correct on light and
  dark taskbars and in high contrast. The menu follows the Windows mode (`SystemUsesLightTheme`, the taskbar's), like
  the icon: it opens from the taskbar, next to Windows' own tray flyouts, so an app-mode menu would clash with a dark
  taskbar. The menu looks like Windows 11's own new menus (owner-drawn items, round corners; Haik liked the
  `p1-tray-menu-look` pictures, 2026-11-03), and each item has a small icon, as in those menus (Haik's ask).
- The tooltip and the menu name what holds the PC awake.
- First run: a notification that offers to pin the icon (a new tray icon lands in the `^` overflow: seen in the tray
  spike, `research.md`, "First add lands in the overflow").
- When Claude's file format cannot be read: a notification "update Wakeshot". The same for a readable status file
  with an unknown `status` value, which counts as working while the session process lives. It shows once per Wakeshot
  start for each agent, plus a log line; the tooltip does not repeat it (Haik, 2026-11-03).
- When a restore fails: a warning notification.
- No "Pause", no icon style choice, no "Efficiency mode", nothing that touches other apps.
- No telemetry. The log stays on the PC.

### Distribution (free options only)

- Public repo `HaikAsatryan/wakeshot` from Phase 1, MIT. Fully public from the first push: docs, planning, reviews
  and spikes go public as they are; secrets never (a secret scan before each push) (Haik, 2026-11-03). Before the
  public launch (winget), a cleanup phase leaves a short, clean README and `CLAUDE.md`, removes internal planning and
  junk, and squashes the history to one initial commit. A GitHub Pages page at v1.
- Velopack pack id `HaikAsatryan.Wakeshot` (install folder `%LocalAppData%\HaikAsatryan.Wakeshot`); data folder
  `%LocalAppData%\Wakeshot`, outside the install folder (Haik, 2026-11-03). Whether Velopack accepts a dot in the pack
  id: not verified; Phase 6 checks it before the first release.
- Velopack: per-user install without admin, auto-update from GitHub Releases, applied only when nothing holds and no
  setting is overridden. The install, update and uninstall hooks restore power settings from the journal. A
  `winget upgrade` during `wakeshot run` still kills the wrapper; the README says so.
- winget first. scoop later if cheap. Microsoft Store only if uninstall can be made safe (MSIX has no uninstall hook,
  so a hold active during uninstall would leave the lid set to Do nothing).
- Signing: v1 ships unsigned. Apply to SignPath Foundation once the project has users; signed builds then show
  "SignPath Foundation" as publisher.

### Answers to the track plan questions (Haik, 2026-11-03 18:09: all defaults)

The two Codex and Copilot answers apply when they are built, after v1.

- Codex waiting for approval looks like working (it writes no approval prompt to disk), so it holds with no limit
  while `codex.exe` lives and its turn is open. The battery floor still protects.
- Copilot waiting for approval (`permission.requested`) does not hold, like Claude waiting without Remote Control.
- Chimes honor the Windows sound scheme: with "No sounds", no chime, one log line.
- Notices raised while the session is locked or another user is on the screen wait until the user is back.
  Sounds never wait.
- After `winget upgrade`, Wakeshot is off until the next sign-in or a manual start; the README says so.
- Uninstall: after a good restore the data folder is deleted; after a failed one only the journal and logs stay.
- After Wakeshot updates itself, one notice says so ("Wakeshot updated to <version>"), no sound.
- Uninstall leaves Windows' own record of Wakeshot's tray icons (`NotifyIconSettings`); the README says so.
- A public pre-release tests the release workflow; Phase 8 deletes it.
- Two users on one PC: Wakeshot overrides only while its user's session is on the screen (Phase 1 Question 10).
- A failed restore by the guard or the sign-in restore starts the tray, so the warning shows (Phase 1 Question 12).
- Remote Desktop into the session is not tested before v1 (Haik, 2026-11-03 18:58).
