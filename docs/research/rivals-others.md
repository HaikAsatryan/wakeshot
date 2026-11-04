# Other keep-awake and lid tools

Collected 2026-11-03. LidGuard is covered in a separate report. Marks: **verified** means read in
the source (code, docs, issue text) at the URL given; **unverified** means a secondary source, a summary, or not read
in full.

## Windows

### Insomnia (stanley-projects/Insomnia)

Electron, 7 stars, v1.4.3. Base review is in `docs/research.md`. New here:

- Triggers: Claude Code and Codex by hooks, Cursor by its transcripts, Aider and Ollama by process name, any app the
  user picks from an auto-found list (Start menu, registry, Store apps, desktop shortcuts, or browse to an `.exe`),
  and a manual toggle. Verified (README): https://github.com/stanley-projects/Insomnia
- Claude session rules: ends on `Stop`; otherwise expires after 3 minutes of silence, or 30 minutes while
  `claude.exe` is alive and a turn is open. Verified (README).
- State: purple owl with open eyes when holding, grey owl with closed eyes when not. Tooltip names the reason:
  "Staying awake for: Claude Code + Chrome" (their text uses a dash). Verified (README).
- Bug: an app added by browsing to an `.exe` never triggers, because it stores the full path but compares against
  `tasklist` names. Verified: https://github.com/stanley-projects/Insomnia/issues/3
- Requests: Claude Code inside WSL (#1), a manual hold with a timer "so I don't leave it awake all weekend" for
  Remote Control use (#2). Verified: https://github.com/stanley-projects/Insomnia/issues/1,
  https://github.com/stanley-projects/Insomnia/issues/2

### PowerToys Awake

Microsoft, part of PowerToys. Docs: https://learn.microsoft.com/en-us/windows/powertoys/awake (verified).

- Modes: off (passive), indefinite, for a time interval, until a date and time. Optional "Keep screen on". CLI:
  `--time-limit`, `--expire-at`, `--display-on`, `--pid` (end when that process ends), `--use-parent-pid`. Custom
  tray durations live in `settings.json` as `customTrayTimes`. Verified (docs).
- Does not touch power plans; it holds `SetThreadExecutionState(ES_SYSTEM_REQUIRED | ES_CONTINUOUS)` on one
  long-lived thread. Lid close is not handled. Verified (docs; code in the Rust rewrite branch,
  https://github.com/dend/PowerToys/blob/dev/awake-rust/src/modules/awake/Awake/awake/src/platform/power.rs).
- Lock screen: stops keeping the display on when locked, keeps the system awake. Verified (docs, PR
  https://github.com/microsoft/PowerToys/pull/49388).
- Tray icon: one icon per mode (passive, timed, expirable, indefinite) plus a tooltip. Verified (docs). Light and dark
  aware monochrome icons are only a pending PR (default off). Verified:
  https://github.com/microsoft/PowerToys/pull/49222
- A Rust rewrite (closed 2026-10-12, not merged) reports startup 130 ms to 16.5 ms and private memory 14.3 MB to
  2.0 MB versus C#. It has a pure `awake-core` state machine crate, `windows-sys` tray with retry and
  `TaskbarCreated` re-add, dark context menus through uxtheme ordinals 135/136, `WM_WTSSESSION_CHANGE` lock
  tracking, and handles `WM_POWERBROADCAST`. CLI `for 1h30m`, `until 18:00`. Verified (PR text and branch files):
  https://github.com/microsoft/PowerToys/pull/51181. MIT licence. Good reference code for Wakeshot.
- Complaints:
  - Request held, PC still sleeps after a manual sleep and resume: `powercfg /requests` shows the SYSTEM request but
    Windows ignores it until the request is toggled off and on. Fixed around v0.98 by re-asserting the request on
    power events. Verified: https://github.com/microsoft/PowerToys/issues/44286,
    https://github.com/microsoft/PowerToys/issues/32544
  - "Goes to sleep after about a day" on Modern Standby laptops while the request is shown as held; one user fixed
    it only by turning Modern Standby off in the registry. Maintainers blame other apps or policy. Verified:
    https://github.com/microsoft/PowerToys/issues/32005
  - Tray icon missing after login or unlock, comes back after toggling the module. 70 comments. Verified:
    https://github.com/microsoft/PowerToys/issues/34148, https://github.com/microsoft/PowerToys/issues/35250
  - "Keeps PC awake after being disabled", even with no Awake process. Verified, cause unclear:
    https://github.com/microsoft/PowerToys/issues/16751
  - Wanted: custom durations, an expiry picker in the tray, and "attach to a process" from the UI (proposed flyout).
    Verified: https://github.com/microsoft/PowerToys/issues/49141
  - Enterprise admins see keep-awake as a way to dodge screen lock policy and ask for a policy to disable
    "indefinite". Verified: https://github.com/microsoft/PowerToys/issues/40546

### Don't Sleep

Freeware, portable, by SoftwareOK. Triggers: timer, network traffic threshold, CPU load. Blocks sleep, hibernate,
shutdown, restart, log off, monitor off, screen saver. Does not change power settings. Verified (vendor page):
https://www.softwareok.com/?seite=Microsoft/DontSleep. No battery or lid feature found. Unverified (page lists
none). Teaches little beyond "network and CPU idle triggers exist and users want them".

### Caffeine (Zhorn Software)

Old freeware. Default method sends a fake F15 key every 59 s; `-stes` uses `SetThreadExecutionState` instead.
Verified (vendor page): https://www.zhornsoftware.co.uk/caffeine/

- Conditions worth noting: `-onac` (only on AC power), `-cpu:xx` (until CPU drops below xx%), `-watchwindow:xx`
  (while a window title contains xx), `-notwhenlocked`, `-activeperiods:0900-1659`, `-exitafter`, `-activefor`.
  Verified (vendor page).
- Control of a running copy from the command line: `-appon`, `-appoff`, `-apptoggle`, `-appexit`, `-replace`.
  Verified. Same idea as a `wakeshot` CLI talking to the tray app.
- Icon: coffee pot, full or empty; `-lighticons` and `-darkicons` switches exist because it cannot follow the theme
  by itself. Verified.
- Fake key presses are the trap: they reset the user idle timer, so the screen saver and auto-lock never start.

### Small GitHub tools

Most are 0 to 30 stars and days or weeks old (searched 2026-11-03 with `gh search repos`). Only ones with something
to teach:

- **WakeKeeper (SurajsinghBayas/wakekeeper)**, Windows, .NET 8 WinForms, 0 stars. Verified (README):
  https://github.com/SurajsinghBayas/wakekeeper
  - Triggers: indefinite, timed, until a time, while an app runs, "until downloads finish", plus auto-start rules on
    app running, AC or battery, Wi-Fi, schedule, idle time, CPU load, or a mounted drive.
  - Lid mode sets the lid action to Do nothing for the session and restores it "even after a crash". Battery guard:
    stop below a charge level or when unplugged, restart when plugged in. Shows a notice when company policy blocks
    the lid change. Tray: a cup, filled while a session runs; click starts or stops.
  - Help menu item "Restore Windows power settings" as a manual escape hatch.
  - Also nudges the mouse so chat apps show "Available". This is the jiggler pattern admins hate.
  - CI runs the real app on Windows Server 2022 and 2025 and checks `powercfg /requests`, lid mode and crash
    recovery, with screenshots of every menu.
- **claude-keep-awake (AxGord)**, Claude Code plugin in the official community marketplace, macOS, Linux and Windows,
  3 stars. Verified (README): https://github.com/AxGord/claude-keep-awake
  - Releases while Claude waits for the user (permission prompt, `AskUserQuestion`, plan approval, after `Esc`,
    after an API error). Detects aborted turns by reading the transcript tail: a last message
    `[Request interrupted by user]` or a line with `"isApiErrorMessage": true` means idle.
  - Holds through a 5-hour usage-limit stop until the reset (bounded at 6 h) so auto-continue can run.
  - Keeps awake while a background task from `run_in_background` still runs after the turn ends.
  - Sessions keyed by CLI pid, and checks the pid still belongs to `claude` (pid reuse).
  - Self-limits: exits when all sessions die, or after 2 h with no hook.
  - macOS: releases after 30 s with no network ("Claude can't reach the API anyway").
  - macOS lid: chime on close and open only while it is overriding the lid; backlight set to 0. Silent when the Mac
    takes the normal sleep path.
  - Windows path is `SetThreadExecutionState` from PowerShell, display and system. No lid on Windows.
- **Caffeinator (Ublaze)**, Windows, Python, 3 stars. Per-app rules (display, system or both), window title regex,
  CPU threshold, "only when in foreground", pauses when the screen is locked, 30 s cooldown after an app exits so a
  restart does not flap, `diagnose` command. Uses `PowerCreateRequest`, visible in `powercfg /requests`. Verified
  (README): https://github.com/Ublaze/Caffeinator
- **claudfeine (maxgfr)**: a wrapper, `claudfeine` = `claude` with a keep-awake around it. macOS `caffeinate -dims`,
  Linux `systemd-inhibit --what=sleep:idle`, Windows `SetThreadExecutionState` in a `finally` block. Verified
  (README): https://github.com/maxgfr/claudfeine. Same idea as `wakeshot run -- <command>`.
- WideAwake (JPacoch), StayLit, DozeGuard: plain toggles or unrelated (StayLit is a screen-on helper, DozeGuard is an
  anti-doze timer). Nothing to copy. Verified (repo descriptions only).

## macOS

### Amphetamine

Free, Mac App Store, the reference app for triggers. Source for all points: the App Store page,
https://apps.apple.com/us/app/amphetamine/id937984704 (verified, read 2026-11-03).

- Sessions: indefinite, timed, until a clock time, while a file downloads, while an app runs. Extend a running
  timed session from the menu.
- Triggers (start a session automatically when all chosen criteria hold): external display, USB or Bluetooth
  device, app running or frontmost, battery charging or above a level, power adapter on or off, IP address, Wi-Fi
  network, VPN, DNS servers, audio output in use, drive mounted, CPU above a level, idle time, schedule. Battery and
  power criteria combine with AND or OR.
- Safety: end the session when battery is low. Lock the screen after inactivity during a session. Allow display sleep
  while locked.
- Closed-display mode: on Apple Silicon needs a helper script and admin authentication, or a separate "Power
  Protect" download, because plugging or unplugging power during a closed-lid session otherwise breaks it.
- State: custom menu bar icons, optional time-left text, start and end notifications, optional sound on each state
  change, dark mode support. Hot keys and AppleScript.
- Complaints in the visible reviews: "while file is downloading" is confusing with browser temp files (Firefox `.part`
  and 0 KB files) and once ended mid-download; App Store sandbox hides background processes from the app trigger;
  notification sound settings stayed after the sounds were removed.

### KeepingYouAwake

macOS, 7,027 stars, wraps Apple's `caffeinate`. Verified (README): https://github.com/newmarcel/KeepingYouAwake

- Deliberately no closed-lid support "out of thermal considerations". Verified (README).
- Battery floor: deactivate below a user-set battery level, added in 1.4 after a request. Verified:
  https://github.com/newmarcel/KeepingYouAwake/issues/24
- Asked for and still open: "disable while screen is locked" (users lock and walk away, battery drains, or the Mac
  wakes at night and never sleeps again), "activate when plugged in", AppleScript toggle. Verified:
  https://github.com/newmarcel/KeepingYouAwake/issues/98, https://github.com/newmarcel/KeepingYouAwake/issues/104,
  https://github.com/newmarcel/KeepingYouAwake/issues/43
- "Not working" reports often turned out to be the menu bar icon hidden behind the notch or a full menu bar.
  Verified: https://github.com/newmarcel/KeepingYouAwake/issues/220
- Holds the screen on too; users who lock the Mac wanted only system awake. Verified:
  https://github.com/newmarcel/KeepingYouAwake/issues/115

### Insomnia (krishhgg/Insomnia)

macOS 26, Swift, 1,896 stars, "timed agent sessions" with the lid closed. Verified (README):
https://github.com/krishhgg/Insomnia

- Trigger is a timer only (days, hours, minutes). Click the countdown to add time, press and hold to end early.
  "Give the session an end time" is the product idea.
- Lid close uses `sudo pmset -a disablesleep 1` through an installed sudoers rule. On lid close: display and keyboard
  backlight off (saved first), audio muted, Low Power Mode on, chat apps frozen (`SIGSTOP`: Slack, WhatsApp,
  Discord). Lid open undoes it.
- Safety defaults: on battery, Low Power Mode below 40% and end below 10%. Heat rules: Low Power Mode when seriously
  hot, end at critical heat. Warning banner: "Use a stable, well-ventilated surface, not a closed bag."
- Recovery: a journal file written before each change, plus a `launchd` agent every minute that undoes changes if
  the app is gone after the end time. Quit refuses while a change is still not undone. The menu shows a recovery
  warning when an undo failed.
- Extras: join the iPhone hotspot when Wi-Fi drops; type `continue` into a marked tmux pane after 90 s offline;
  offer to relaunch Chrome without background throttling because closed-lid windows count as hidden.
- Issues: users ask for an "Auto" mode that holds only while an agent works and shows which job holds it (#57), heat
  safety in a backpack (#55), temperature control (#52), a hold-to-end control that misfires (#60). Verified:
  https://github.com/krishhgg/Insomnia/issues/57, https://github.com/krishhgg/Insomnia/issues/55,
  https://github.com/krishhgg/Insomnia/issues/60
- One comment on #57: an agent waiting out a usage limit retries every 5 minutes, so "idle between turns" is not
  "done". Verified (comment on #57).

### Lungo and Theine

- **Lungo** (Sindre Sorhus): durations or until off, let the display sleep, pause while the screen is locked,
  deactivate when switching to battery, Shortcuts action, URL scheme `activate/deactivate/toggle` with hours and
  minutes, a CLI. No app or schedule triggers by design. Closed lid only with charger and external display (App
  Store rules). Verified (vendor page): https://sindresorhus.com/lungo
- **Theine**: one-click toggle, hot key, "auto-deactivate on low battery", "lockscreen allows sleep", no lid.
  Unverified (search summary of the App Store listing): https://apps.apple.com/app/theine/id955848755

### Claude Code caffeinate on macOS

- The CLI spawns `caffeinate -i -t 300`, kills and respawns it every 240 s, releases 30 s after work ends. Users
  report it sometimes holds while idle (waiting at a question) and keeps a closed lid awake; they ask for an off
  switch. Verified: https://github.com/anthropics/claude-code/issues/21432
- A user lost a session: held on battery until 0%. macOS writes the hibernate image only when it sleeps, so it never
  hibernated and hard-powered-off. Suggested fix: do not hold on battery by default. Verified (comment on #21432).
- The relay kills the old `caffeinate` before starting the new one, so there is a short window with no assertion.
  With the display already off, the Mac sleeps in that window and the CLI cannot restart its own relay. Still present
  in 2.1.266 per comments. Verified: https://github.com/anthropics/claude-code/issues/81832
- Windows CLI holds nothing (#96679, in `docs/research.md`).

## Linux

### GNOME Caffeine extension

801 stars, a quick settings toggle. Verified (README and settings schema):
https://github.com/eonpatapon/gnome-shell-extension-caffeine

- App triggers: a list of apps (`inhibit-apps`) with three modes: while running, while focused, while on the active
  workspace. Also: enable while any app is fullscreen, enable while an app plays media (MPRIS). Timer with custom
  durations, restore state after restart, notifications on change, filled or empty cup icon. CLI by flipping a
  `gsettings` key.
- Complaints:
  - Inhibits sleep through gnome-session, which also blocks the user's own manual suspend ("Operation inhibited ...
    user session inhibited"). Verified: https://github.com/eonpatapon/gnome-shell-extension-caffeine/issues/353
  - Sometimes still blocks autosuspend after being turned off, until reboot. Verified:
    https://github.com/eonpatapon/gnome-shell-extension-caffeine/issues/222
  - With two trigger apps, closing the first turned it off while the second still ran (not reproduced by the
    author). Verified: https://github.com/eonpatapon/gnome-shell-extension-caffeine/issues/7
  - Users expect it to stop lid suspend too; it does not. Verified:
    https://github.com/eonpatapon/gnome-shell-extension-caffeine/issues/16

### caffeine-ng

Tray app (AppIndicator), auto-activates while a fullscreen app runs, works with the `org.freedesktop.ScreenSaver`
API, xautolock, DPMS. Verified (README): https://codeberg.org/WhyNotHugo/caffeine-ng. Nothing new to copy.

### KDE Plasma

The battery applet has "manually block sleep and screen locking" and lists apps that hold an inhibition with their
reason. Newer versions add buttons to release or permanently block one app's inhibition, stored as
`BlockedInhibitions` in `~/.config/powerdevilrc`. Unverified (merge request and forum thread, release not checked):
https://invent.kde.org/plasma/plasma-workspace/-/merge_requests/4341,
https://discuss.kde.org/t/how-can-i-see-which-apps-are-prevented-from-blocking-sleep-and-screen-locking/43615.
Lesson: the user can see who is holding the PC awake, and veto it.

### systemd-inhibit and logind

Verified (man pages): https://www.freedesktop.org/software/systemd/man/latest/systemd-inhibit.html,
https://www.freedesktop.org/software/systemd/man/latest/logind.conf.html

- `--what=` takes `idle`, `sleep`, `shutdown`, and `handle-lid-switch` (and other key handlers). Modes: `block`,
  `block-weak` (not enforced against the lock owner's own requests), `delay`.
- Lid: `HandleLidSwitch` defaults to suspend, `HandleLidSwitchDocked` to ignore, `HandleLidSwitchExternalPower` is
  ignored unless set. `LidSwitchIgnoreInhibited=yes` by default, so a normal sleep lock does not stop lid suspend;
  only a `handle-lid-switch` lock does.
- So on Linux, a later Wakeshot port can take `idle` (not `sleep`) for "keep awake" so manual suspend still works,
  and `handle-lid-switch` for lid mode, without editing any config file. Inference from the man pages, not tested.

### Open checks for a later Linux port

Unverified: from model knowledge in a chat on 2026-11-03, not checked against docs or tested.

- Target desktops, not distros. The big distros all use systemd-logind, but idle sleep is decided by the desktop:
  GNOME's settings daemon, KDE PowerDevil, xfce4-power-manager, `swayidle` or `hypridle`.
- GNOME may run auto-suspend itself and honor only gnome-session inhibitors. Then the logind `idle` lock above would
  not stop it. Test this first.
- KDE PowerDevil may handle lid close itself, so a logind `handle-lid-switch` lock may act differently than on GNOME.
- `org.freedesktop.portal.Inhibit` has suspend and idle flags and may cover GNOME and KDE with one code path. Check
  how each desktop maps the flags.
- Tray: GNOME has no tray by default. Ubuntu ships its AppIndicator extension turned on; Fedora does not.
- WSL2 can build and unit test the Linux code, and take logind locks once systemd is on, but it has no lid, idle sleep
  or suspend. Sleep and lid tests need real hardware; a live USB of another desktop on the same laptop is enough.

## Ideas for Wakeshot

1. Tooltip and menu name what holds the PC awake: "Awake for: Claude Code (2 sessions), ffmpeg" (Insomnia, KDE).
2. Re-assert the power request on resume from sleep and on any power broadcast, not only at start (PowerToys fix).
3. Never drop the request before taking the new one; overlap them (Claude Code #81832).
4. Handle `TaskbarCreated` and retry adding the tray icon at login until the notification area is ready (PowerToys).
5. First-run tip to pin the icon out of the `^` overflow, since a hidden icon reads as "not working".
6. Treat a session waiting on the user (permission prompt, question, after `Esc`, after an API error) as idle
   (AxGord plugin, Claude Code #21432).
7. Read the last line of the session transcript to catch aborted turns that never set the status back (AxGord).
8. Consider holding through a 5-hour usage-limit stop until the reset, with a hard cap (AxGord, Insomnia #57
   comment). Product call for Haik.
9. Check a pid still belongs to the expected exe before trusting it (pid reuse; AxGord).
10. Cooldown of about 30 s after a trigger app exits so a restart does not flap the state (Caffeinator).
11. App rules match the exe name and also the full path the user browsed to (Insomnia #3).
12. App rule modes worth offering later: while running, while in the foreground (GNOME Caffeine, Amphetamine).
13. A "hold for 2h" item in the tray, plus "add 1 hour" while a hold runs (Insomnia #2, Amphetamine extend,
    krishhgg Insomnia).
14. Lid mode safety on top of the 20% floor: release on unplug if the lid is closed and battery is low. A user-mode
    temperature reading does exist (PDH thermal zone, `research.md:478-487`), but Wakeshot has no heat guard and no
    lid-closed time limit (decisions 2 and 3, 2026-11-03).
15. Sounds only when they mean something: a short chime on lid close only while Wakeshot is overriding the lid
    (AxGord). Silent otherwise.
16. A "Restore my power settings" menu item as a manual escape hatch (WakeKeeper).
17. Show a warning in the menu when a restore failed, and refuse to quit silently while one is pending (krishhgg
    Insomnia).
18. Notice when company policy blocks the lid change, instead of failing silently (WakeKeeper).
19. CLI controls the running tray app: `wakeshot on|off|toggle|hold 2h|run -- cmd` (Zhorn `-appon`, claudfeine,
    PowerToys `--pid`).
20. Release when the network has been down for a while (Claude cannot work offline). Optional, from AxGord.
21. Claude Code inside WSL writes its session files in the Linux home, not `%USERPROFILE%`. Decide whether to read
    `\\wsl$` paths (Insomnia #1).
22. Reference code in Rust: the PowerToys Awake rewrite branch (MIT) for tray retry, dark menus, lock tracking, and a
    pure state machine crate.

## Traps seen in real use

1. Windows ignores a held SYSTEM request after a manual sleep and resume until it is re-taken (PowerToys #44286,
   #32544).
2. On some Modern Standby laptops the PC still sleeps "after about a day" with the request held; maintainers blame
   other apps or policy (PowerToys #32005). Log what is held and when the PC slept, so a report can be checked.
3. Tray icon missing after login, unlock, or an Explorer restart (PowerToys #34148, 70 comments).
4. "Stuck on": PC will not sleep after the tool is turned off, sometimes until reboot (PowerToys #16751, GNOME
   Caffeine #222). Release everything on off, on quit, and on crash.
5. Holding on battery with nobody there drains to 0% and loses the work, because no hibernate happens (Claude Code
   #21432 comment).
6. A keep-awake that stays on while locked drains the battery or keeps a woken Mac up all night (KeepingYouAwake #98).
7. Release then re-acquire leaves a gap where the PC sleeps (Claude Code #81832).
8. Holding while an agent only waits for the user keeps the laptop awake for nothing (Claude Code #21432).
9. Fake key presses or mouse nudges (Zhorn Caffeine default, WakeKeeper option) defeat screen lock; admins treat
   keep-awake tools as jigglers and ask for policy controls (PowerToys #40546, #49141 comments).
10. Blocking "sleep" instead of "idle sleep" also blocks the user's own Sleep button (GNOME Caffeine #353). On Windows
    `SetThreadExecutionState` does not block user-initiated sleep (PowerToys docs).
11. Lid-closed laptops in a bag overheat; every serious lid app warns about it or asks for heat rules (krishhgg
    Insomnia README and #55, KeepingYouAwake README).
12. Plugging or unplugging power during a closed-lid session can break the override (macOS: Amphetamine needs a
    script, AxGord documents a forced micro-sleep). Windows equivalent not checked: test AC/DC switches with the lid
    closed.
13. Download triggers based on files are fragile (browser temp files; Amphetamine reviews).
14. Process name versus full path mismatch in app rules (Insomnia #3).
15. Multiple trigger apps: the first one closing must not release while another still runs (GNOME Caffeine #7).
16. Closed-lid browsers throttle hidden windows; web-based agents may stall even when the PC is awake (krishhgg
    Insomnia README). Not checked on Windows.
