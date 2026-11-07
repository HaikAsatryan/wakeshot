# Product

What Wakeshot v1 does and the rules it follows. A free personal project, MIT license (GitHub
`HaikAsatryan/wakeshot`).

## The job

Keep the PC awake only while something needs it, then let it sleep as usual. "Something" is an AI agent at work
(Claude Code first) or a long command started with `wakeshot run`. On top: keep awake on demand, like PowerToys
Awake, so nobody needs two tools. Keeping a laptop working with the lid closed is one more feature, not the core.

The user must always understand what happens to their laptop. The tray icon shows the state. A notification or a
sound explains every action Wakeshot takes on its own. The routine start and end of a hold, with the power settings
Wakeshot writes and puts back each time, show in the icon and tooltip only.

When no work is going on, the laptop behaves as if Wakeshot were not there: the user's own settings decide. A case the
rules below do not settle follows this rule.

## Decided

### Platform and stack

- Windows 11 only. Native x64 and ARM64 builds. English only.
- Rust with `windows-rs`. Reason: tiny always-on footprint, no runtime, decision core is portable. Runner-up: C# .NET
  10 Native AOT. Switch when Rust reviews get too slow, or when a rich window is needed before `windows-reactor`
  (Microsoft's WinUI library for Rust, preview in 2026) is stable.
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
  - Some ends of a background task write no notification to the transcript (Claude Code 2.1.317 code; not checked
    live). A task stopped with `TaskStop` ends at that call's result. A Monitor that is not persistent holds only
    until its own timeout (at most 1 hour). A task stopped from the app's UI ends when Claude Code queues its stop
    notice, which it writes to the transcript at once (2.1.317 code; not checked live).
- **Other agents**: Codex CLI, Copilot CLI, Copilot in VS Code (its Copilot harness chats, which are Copilot CLI
  sessions and hold through the Copilot source; `research/copilot-vscode-signals.md`), and OpenCode 2.x. Not in v1,
  each because its end of work is not always seen ("No guessing" below):
  - VS Code's own Local chats: a Local chat whose window closes mid-turn stays "working" on disk until VS Code exits,
    and nothing on disk shows the end. Revisit when VS Code writes the end at a close.
  - Antigravity: Esc fires no hook, and the cancel line in `agy`'s own log was checked for one `agy` version (1.5.2),
    while `agy` releases almost daily, so it would hold nothing most of the time. The tray neither watches it nor
    adds its hook folder; a start, `wakeshot hook antigravity remove` and the uninstall remove a folder an earlier
    build added. Revisit when Esc fires a hook, or the cancel line stays the same across `agy` releases.
  - Cursor, editor and CLI: a chat deleted while it runs sends no `stop`, only the `sessionEnd` that Ctrl+F4 also
    sends (whose turn goes on), so the hold would last until the window closes. The tray neither watches it nor adds
    its hook folder; a start removes the folder an earlier build added, as the uninstall does. Revisit when a chat
    deleted mid-turn sends its own end.

  v1 supports OpenCode 2.x only, and says "OpenCode 2.x". No agent logos in the video, README or site (their owners'
  rules need approval or are unclear): names only. Wakeshot may open `opencode.db` read only and query only the
  session and schedule tables, though the file also holds OpenCode's login tokens, with a test that no other table is
  ever read (`research/opencode-signals.md`). Gemini CLI is dropped: since 2026-06-18 it serves only enterprise and
  paid-API users. Not planned: Aider, Amp, Kiro. Logic may be ported from the MIT rivals with credit. Every agent gets
  a live test; an agent not tested live is marked so in the docs.
- **`wakeshot run -- <command>`**: holds while the command runs. Writes a hold file, a small documented format that
  any tool or script can also write.
- **Keep awake on demand**: see UX.
- Every signal is checked against a fact, not capped by time. A Claude session counts only while its process lives
  (a wait on the process handle), so a crashed session or a leftover file never holds. A timer counts until its due
  time plus a short margin; this also ends a wakeup cancelled with Esc, which writes nothing to disk. `busy` and
  `shell` have no cap while the process lives. A hold file counts while its process lives, so `wakeshot run` ends
  with its command. Keep awake ends at its time or at sign-out. No extra safeguard for another program's bug. Two
  limits stay because they are product rules, not bug guards: Claude waiting for approval holds at most 1 hour (the
  `waiting` rule above), and the battery floor. Codex and Copilot: see "Detailed rules".
- No guessing: if Wakeshot cannot see when an agent's work ends, it does not support that agent or that case, rather
  than end the hold on a timer or keep it on a guess. Claude's cancelled wakeup stays as it is: Esc writes nothing, so
  the hold lasts until the due time Claude wrote, at most 1 hour.
  The Antigravity code, out of v1 ("Other agents" above), stays for a later version under the same rule: Wakeshot may
  read `agy`'s own log for its cancel line only (the file also holds prompts: nothing else is kept or logged, with a
  test); if the log stops looking as tested, Wakeshot stops holding for Antigravity and shows "update Wakeshot". The
  CLI only; the IDE stays out until its own check passes. As built (2026-11-06), the Antigravity CLI holds from a
  model call to its `Stop`, to the cancel line (Esc, Ctrl+C; seen at the next read, within 30 s) or to `agy`'s end;
  tested `agy` versions only (1.5.2). Background commands are not held: no exact end in every case (`fullyIdle`
  ignored). The IDE's hooks write nothing.
- Later, not v1: rules for any app ("while ffmpeg runs"). Claude Code inside WSL: its session files carry Linux pids,
  which Windows cannot check, so stale files would hold forever. Revisit with a helper inside WSL.
- No plugins and no scripting. A new built-in source is new code.
- Hooks: for an agent with no reliable file signal, Wakeshot may add its own hook files in a folder it owns that the
  agent loads (a plugin or extension folder), never by editing the user's own settings, and never for Claude Code. It
  adds them by itself when it finds the agent, says so once in a notice, lists them in the README, and removes them
  at uninstall. v1 adds no hooks: its two hooked agents, Cursor and Antigravity, are out of v1 ("Other agents"
  above). A v1 start removes the hook folder an earlier build added, by the uninstall's rules, and shows no notice.

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
  AC, so an unplug or a short AC drop on a USB-C dock never meets that rule first. The value is Never. Tested with
  Never only, lid open: on battery a held request stayed honored for 16 minutes with no input, past the point where
  the test laptop's own 10 minute timeout had ended it. A finite value only moves the point where requests end (5
  minutes after it, by Microsoft's text; not tested).
- **Never lower security.** When the lid closes while Wakeshot holds, Wakeshot locks the PC: Windows would have
  slept, and with the lid action Do nothing it does not lock by itself (tested). When the screen turns off for idle,
  Windows locks by itself, also while a request is held (tested with sign-in "Every Time"), so Wakeshot adds no idle
  lock. Only with Keep screen on, where the screen never turns off, Wakeshot locks when idle passes the user's own
  sleep timeout. No lock if "require sign-in after sleep" is Never; which stored value means Never is not known yet
  (Windows Hello greys the choice out on the test laptop), so until a test shows it, Wakeshot locks. There is no test
  account for it, so v1 always locks at lid close while holding. No lock on lid close in docked use (the user's own
  lid action is Do nothing). Unplugging a closed laptop that then keeps running only because of Wakeshot locks it
  too, once it has been on battery for 30 seconds (the test laptop's dock dropped AC for 5.3 s and 6.3 s, so a
  5 second wait locked a user working on the external screen). If that lock fails, or Windows does not report the
  session locked within 5 seconds, Wakeshot tries once more; if that fails too, it steps back as at the battery floor
  (releases every request, restores settings) and puts the laptop to sleep as at work end (below), so a closed laptop
  never runs unlocked in a bag. The step back lasts until the session locks or the lid opens; then the hold runs
  again. The notice about the failed lock is shown again when the user is back at the open laptop, since with the lid
  closed nobody may have seen it. A hold that starts while the lid is already closed is no lid close, so it does not
  lock: the user may be working on an external screen.
- Sounds wherever they give a useful hint:
  - Lid closed while Wakeshot holds: a short chime says the laptop keeps running.
  - Work ended with the lid closed: a short "going to sleep" sound right before Wakeshot puts the laptop to sleep.
  - Battery floor reached.
  - No sound for: a failed restore (the warning notification is enough), a hold that starts or ends with the lid
    open, Keep awake running out, the lid opening, AC lost or back (except when it leaves a closed laptop running on
    Wakeshot's Do nothing: then as a lid close, with the lock; under the battery floor the lock comes with the battery
    floor sound instead, "Battery floor" below), the user changing a setting Wakeshot overrides, and
    "update Wakeshot". These get a notification or the icon only.
  - Every sound plays on the default audio device, unless that device is a screen's audio (HDMI or DisplayPort, often
    with no speakers); then on the laptop speakers. Headphones stay as they are. The chime was heard at two lid
    closes with the lock on (`research.md`, "Lid-close sound").
- After a closed-lid hold, once the lid is open and the user is signed in (at the first unlock, or when the lid opens
  while signed in), a notification says how long the PC kept running with the lid closed and what held it. A docked
  user who signs in on the external screen with the lid still closed gets it when the lid opens.
- **Battery floor**: the Windows energy saver threshold; 20% when it is Never or Always. No Wakeshot setting. On
  battery below the floor, Wakeshot steps back once the PC has been on battery for 30 seconds: it releases every
  request, restores settings and notifies, so Windows rules apply. If AC comes back within the 30 seconds, nothing
  happens: no step back, no chime, no notice (without the 30 seconds, every short dock AC drop in checks on the test
  laptop on 2026-11-05 gave a chime and a notice, and one notice showed late, on AC; the same 30 seconds as the lock
  after an unplug). During the 30 seconds every hold keeps holding, and no setting is restored or written again. They
  count from the AC loss after AC that lasted 30 seconds: AC that comes back for less does not start them again, so a
  charger that flaps, on battery under 30 seconds at a time, still steps back 30 seconds after its first loss; each
  later blip of AC then ends the step back, and the next loss steps back again at once, with its sound and notice (a
  quieter choice would end a step back only once AC has lasted 30 seconds too). A battery that falls under the floor
  after 30 seconds on battery steps back at once, and so does a Wakeshot that starts on battery under the floor:
  there is no AC drop to wait out. An unplug with the lid closed brings the lock after an unplug (above) at the same
  moment: the lock goes first, with the battery floor sound only, since the lid-close chime says the laptop keeps
  running. That lock is watched and tried once more as at a lid close, also once the step back's restore has landed,
  and a lock after an unplug that still waits when the step back comes stays, since the closed laptop runs on. If the
  lid is closed, it then puts the laptop to sleep the same way as at work end (below; sleep, not hibernate), after
  the same 5 minutes with no input, so someone typing on an external screen with the lid closed is not put to sleep;
  at the step back, 30 seconds after the AC loss, Windows may sleep it at once instead (not known; work end, below).
  If the user's own battery lid action is Do nothing, it does not act, the same rule as at work end. Right before it
  acts, it checks again that the lid is closed and the PC is on battery under the floor: a USB-C dock can drop AC for
  a few seconds (seen on the test laptop). When AC comes back, the hold resumes.
- With the lid closed, Wakeshot holds for as long as the work runs: no time limit and no heat guard. Someone who sets
  the lid to Do nothing by hand and puts the laptop in a bag gets no heat check either, and the hardware's own
  protection applies the same in both cases: the CPU slows itself at its limit (not tested here), and Windows shuts
  down at the critical trip point (113 C on the test laptop, `research.md`, "Limits: Kernel-Power 125").
- When work ends with the lid closed, Wakeshot puts the laptop to sleep once there has been no input for 5 minutes,
  so someone typing on an external screen with the lid closed is not put to sleep. The rule: when Wakeshot is done,
  the laptop ends up where Windows would have put it, and with the lid closed that is sleep. It must act itself:
  putting the lid action back does not sleep a laptop whose lid is already closed (tested, with no power source
  change). Right after one it did: in a check on the test laptop (2026-11-05) the battery floor's step back put the
  user's own Sleep back 1 second after the closed laptop was unplugged, and Windows slept it 0.4 seconds later, so
  the 5-minute route never ran; whether the source change or the restore made Windows act is not verified. With the
  battery floor's 30 seconds the restore comes 30 seconds after the unplug, and it is not known whether Windows still
  acts then or the 5-minute route runs; both are safe. After a release the laptop runs until the user's own screen
  timeout turns the screen off and the sleep timeout has passed (tested with the lid closed on battery). On Modern
  Standby, Windows gives apps no call that starts sleep (Microsoft Q&A; on the test laptop `SetSuspendState(FALSE)`
  hibernates, tested), so Wakeshot uses Windows' own idle sleep: after the sound it sets the screen and sleep
  timeouts of the plan in use, on AC and DC, to 1 minute where the user's own is longer, marked in the journal as
  Wakeshot's. The screen one is needed: on the test laptop the sleep came when the screen timeout turned the screen
  off, not at the sleep timeout (tested with the lid closed on battery: Windows slept by itself, Modern Standby,
  locked at wake). They go back at lid open, at an unlock, when a hold starts, at input (then the 5 minutes start
  again), and at every other restore point; not at a wake alone, so a laptop that wakes with the lid closed sleeps
  again. If the laptop has not slept 5 minutes later, with the lid still closed and no input since, Wakeshot
  hibernates it, also when a restore in those minutes put the user's own timeouts back (`wakeshot restore` from a
  script; a menu click is input), so a closed laptop never runs on for hours. This way a lid close's sleep is matched
  even when another app holds the PC awake: `SetSuspendState(TRUE)` after `IsPwrHibernateAllowed` and enabling
  SeShutdownPrivilege, with the lid checked right before the call. If hibernate is not allowed or the call fails:
  notify; the short timeouts stay in force, so Windows sleeps the laptop once nothing holds it; they go back at input
  or an unlock as above, and come again, with the sound, after the next 5 minutes with no input. Until the lid opens,
  Wakeshot then locks the laptop, with the lid-close chime, whenever it has had no input for 5 minutes and is
  unlocked, at most every 5 minutes, so it does not run unlocked for hours. A notice shown while the lid was closed
  and the session unlocked, which nobody may have seen, is shown again when the user is back at the open laptop. If
  the user's own lid action is Do nothing, Wakeshot changed nothing, so it only releases its hold, and it does not
  lock either.

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
7. A journal that cannot be read: Wakeshot tries again every 30 seconds for 2 minutes; a damaged file ends that at
   once. The 2 minutes are run time: a sleep pauses them. Then, on the plan in use, each setting Wakeshot writes that
   holds exactly the value Wakeshot writes (lid Do nothing, battery sleep Never, and the 1-minute screen and sleep
   timeouts of the work end rule) goes to Windows' default for that plan; any other value is the user's and stays.
   The uninstall hook runs the same reset on a damaged journal, since no tray starts after it (log only, no notice).
   For battery sleep that default is 180 seconds on the test laptop (`research.md`, "Windows defaults of the three
   keys"), so a user whose own value was Never gets 3 minutes. The log gets each old and new value, the bad file is
   set aside, and one notice says what was reset and that a user who had their own values should set them again.
   During the 2 minutes Wakeshot neither locks at a lid close nor counts down; if the lid is closed when they end and
   nothing holds, the work end rule puts the laptop to sleep. Rare cost: a user's own Do nothing or Never meets a lost
   journal and goes back to the default, with the notice; with nothing holding, a closed laptop that a crashed
   Wakeshot left at Do nothing may run unlocked for up to about 7 minutes before it sleeps.

The crash gap is accepted: after an OS crash, a power cut or the other cases listed in `architecture.md` ("Gap left"),
nothing restores before the next sign-in, because that would need admin, or until Wakeshot next starts in the rarer
cases listed there. Until then the lid action stays Do nothing and battery sleep Never (or the 1-minute timeouts stay,
after a closed-lid work end). The README says so.

### UX

- No window. Tray icon, tray menu, notifications, sounds, CLI, log file. Everything the CLI controls, the menu
  controls too, except `wakeshot run`, which wraps a command, and `wakeshot diagnostics`, a support command that
  controls nothing: Wakeshot's users run agents in terminals.
- CLI commands: `wakeshot run -- <command>`, `wakeshot keep-awake 30m|1h|2h|4h|on|off [--screen]` (`on` means Until I
  turn it off), `wakeshot automatic on|off`, `wakeshot status`, `wakeshot restore`, `wakeshot log`,
  `wakeshot diagnostics [folder]` (a bug report file for the user to attach; nothing is sent). `keep-awake` needs the
  tray running and says so if it is not. `run` always runs the command, and warns when the tray is not running or
  Automatic is off.
- **Automatic**: on or off. Watches agents and `wakeshot run`. Survives a restart. Starts at login, on by default.
- **Keep awake**: Off, 30 minutes, 1 hour, 2 hours, 4 hours, Until I turn it off. Option "Keep screen on". Works
  whether Automatic is on or off. It is one more hold, so the icon, lid rules and battery floor apply the same way.
  It ends at sign-out or restart, so a forgotten "Until I turn it off" does not survive a reboot.
- Menu sketch (the app's final words may differ):

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

- Tray icon states: holding (a filled light over a line), idle (the same light as an outline), off (the outline struck
  through). Three, not two, so a forgotten Automatic off shows in the tray (`brand.md`). The icon is in Windows' own
  glyph colours; only while holding does the light take the brand colour, Moss, and high contrast draws every state in
  one colour. The menu's status row shows the same glyph. The brand colours are otherwise only in the app icon, the
  page, the README header, the social card and the media. It must look good, be crisp at every display scale, and be
  correct on light and dark taskbars and in high contrast. The menu follows the Windows mode (`SystemUsesLightTheme`,
  the taskbar's), like the icon: it opens from the taskbar, next to Windows' own tray flyouts, so an app-mode menu
  would clash with a dark taskbar. The menu looks like Windows 11's own new menus (owner-drawn items, round corners;
  `research/tray-menu-look.md`), and each item has a small icon, as in those menus.
- The tooltip and the menu name what holds the PC awake.
- First run: a notification that offers to pin the icon (a new tray icon lands in the `^` overflow: seen in a test
  program, `research.md`, "First add lands in the overflow").
- When Claude's file format cannot be read: a notification "update Wakeshot". The same for a readable status file
  with an unknown `status` value, which counts as working while the session process lives. It shows once per Wakeshot
  start for each agent, plus a log line; the tooltip does not repeat it.
- When a restore fails: a warning notification.
- No "Pause", no icon style choice, no "Efficiency mode", nothing that touches other apps. The hooks rule above is the
  one allowed exception (a hook folder of Wakeshot's own that an agent with no reliable file signal loads; never an
  edit of the user's own settings), and v1 uses it for no agent: a v1 start only removes a folder an earlier test
  build added.
- No telemetry. The log stays on the PC.

### Distribution (free options only)

- Public repo `HaikAsatryan/wakeshot`, MIT. Secrets never go public (a secret scan before each push). A GitHub Pages
  page.
- README: long and detailed is fine, but never boring. The top says what it does (the PC stays awake while an AI
  agent or another long job works, then sleeps as usual with the user's settings back; keep awake on demand comes
  with it, so nobody needs a second tool), then the video, then install, super simple, winget first. Then what holds
  the PC awake, as a list that grows: every supported AI agent, A to Z, none first (the focus is not Claude Code
  alone), each tested live, plus any command, hold files and Keep awake. The agents left out ("No guessing") are named
  in Limits, each with its one-line reason; v1 adds no hooks, so the README has no hook lines. Lightweight is enough:
  one measured line deep in the details, no tech or size numbers near the top. A small side by side table against the
  2 or 3 most popular keep-awake options only (many thousands of stars or users), few rows, low on the page. Details
  sit below in folded sections. It says macOS comes if Wakeshot gets popular or gets donations. Pictures of the menu
  and tooltip, light and dark.
- A short promo video (the main cut runs 38 s) with sound and effects, animated in code from the real icon art and
  screenshots, showing only what Wakeshot really does. For the README, the Pages page and the Store listing.
- Velopack pack id `HaikAsatryan.Wakeshot` (install folder `%LocalAppData%\HaikAsatryan.Wakeshot`); data folder
  `%LocalAppData%\Wakeshot`, outside the install folder. Velopack accepts the dot in the pack id: pack, install,
  updates, Setup over an install and uninstall all worked with it (tested, research "Velopack checks for Wakeshot",
  point 1).
- Velopack: per-user install without admin, auto-update from GitHub Releases, applied only when nothing holds and no
  setting is overridden. The install, update and uninstall hooks restore power settings from the journal. A
  `winget upgrade` during `wakeshot run` still kills the wrapper; the README says so.
- winget first. scoop later if cheap. Microsoft Store as well, if adding it and signing are free and the submission
  runs in the release flow without hands; uninstall must still restore (MSIX has no uninstall hook, so a hold active
  during uninstall would leave the lid set to Do nothing). winget, the Store and the app's own updates must never
  fight over one install.
- Signing: v1 ships unsigned. Apply to SignPath Foundation once the project has users; signed builds then show
  "SignPath Foundation" as publisher. No paid certificate (such as Certum Open Source): free paths only for signing
  and distribution.

### Detailed rules

- Codex waiting for approval looks like working (it writes no approval prompt to disk), so it holds with no limit
  while `codex.exe` lives and its turn is open. The battery floor still protects.
- Copilot waiting for approval (`permission.requested`) does not hold, like Claude waiting without Remote Control.
- Chimes honor the Windows sound scheme: with "No sounds", no chime, one log line.
- Notices raised while the session is locked or another user is on the screen wait until the user is back.
  Sounds never wait.
- Notices stay tray banners (`Shell_NotifyIcon`), not registered toasts. They close after Windows' "Dismiss
  notifications after" time and a missed one is not kept in Win+N.
- After `winget upgrade`, Wakeshot is off until the next sign-in or a manual start; the README says so.
- Uninstall: after a good restore the data folder is deleted; after a failed one only the journal and logs stay.
- After Wakeshot updates itself, one notice says so ("Wakeshot updated to <version>"), no sound.
- Uninstall leaves Windows' own record of Wakeshot's tray icons (`NotifyIconSettings`); the README says so.
- A public pre-release tests the release workflow, then is deleted.
- Two users on one PC: Wakeshot overrides only while its user's session is on the screen.
- A failed restore by the guard or the sign-in restore starts the tray, so the warning shows.
- Remote Desktop into the session is not tested.
