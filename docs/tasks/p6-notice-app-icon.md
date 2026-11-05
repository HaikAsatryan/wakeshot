# Show the Wakeshot icon in the notice header

Phase: 6 (tray fix). Status: done (Part A and Part B 2026-11-05; Part B bundled with `p6-screen-reruns`). Needs Haik: Part B only, about 5 minutes at the screen (he looks at two notice
headers). Agent: `wakeshot-worker`. Size: about 100K tokens (Part A about 85K, Part B about 15K).
Written by the orchestrator, 2026-11-05, from `docs/tasks/p6-int-haik-checks.md` "Found outside scope" 5 and 11.
Runs after: `p6-real-install-guard` (both own `packaging/e2e.ps1`). Parallel with: `p2-6-floor-grace` (it owns
`wakeshot-core` and the floor tests in `app/dispatch.rs` and `app/tray_mode.rs`; do not edit those files).
Real laptop: Part A: test installs of a local pack (HKCU keys, the install folder); no power settings. Part B: the
screen, unlocked, Haik at the laptop.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-notice-app-icon\target`.

## Goal

Every balloon of the installed Wakeshot, started from the Start menu, shows a plain white square in its header where
the app icon goes (Haik's 00:50 photo, `C:\Users\haika\Desktop\Screenshot 2026-11-05 005234.png`: read it, never move
or delete it). Started from Start, the tray carries the shortcut's AppUserModelID `velopack.HaikAsatryan.Wakeshot`, and
Windows files its balloons under that id, which has no `HKCU\Software\Classes\AppUserModelId\...` key and so no
`IconUri`. Started another way (the Run value at sign-in), it most likely gets the shell's generated id with the tray
glyph (inferred). Every user sees the white square. Make both start paths show the Wakeshot app icon and the name
"Wakeshot" in the header.

## Pick (orchestrator, for Haik's review; check it against current Microsoft docs first)

- Register the app id the way Windows did itself for the generated id: `HKCU\Software\Classes\AppUserModelId\
  velopack.HaikAsatryan.Wakeshot` with `DisplayName` "Wakeshot" and `IconUri` pointing to a PNG of the app icon (the
  Sunrise plate of `assets/wakeshot.ico`, a size the docs name). Use the values the current docs for notifications
  from unpackaged apps name (for example `IconBackgroundColor`), nothing more.
- The tray sets the same id for itself at start (`SetCurrentProcessExplicitAppUserModelID`), so a start from Run and
  a start from Start behave the same. The dry-run tray never writes or uses the real id (it stays apart from a real
  Wakeshot).
- Who writes the key and the PNG: the install and update hooks, or the tray at each start (that also covers the
  portable zip). Pick one, say why. The uninstall hook removes the key and the PNG.
- If the docs show a better documented way, take it and record why.

## Read first

- `docs/tasks/p6-int-haik-checks.md` "Found outside scope" 5 and 11; `docs/research.md` "Integration checklist with
  Haik" (the notice and icon facts).
- `crates/wakeshot-windows/src/notify.rs`, `hooks.rs`, `crates/wakeshot-tray/src/main.rs`, `assets/`,
  `packaging/pack.ps1`, `packaging/e2e.ps1`; `docs/architecture.md` section 6 "Files"; `README.md` "what an uninstall
  leaves" (about lines 482 to 483).

## Deliverables

### Part A: agent

1. The change above, with unit tests where the code allows (key and PNG writes behind the existing test seams).
2. `packaging/e2e.ps1`: after install, the key and PNG exist with the right values; after uninstall, both are gone.
   Run it once on a local pack. Before each install, the guard of `p6-real-install-guard` must pass (no real
   Wakeshot installed or running).
3. Docs: `docs/architecture.md` section 6 "Files" (only that section; re-read right before the edit, another card
   edits other lines); `README.md` uninstall leftovers: add `Notifications\Settings\velopack.HaikAsatryan.Wakeshot`
   (Windows keeps it after the uninstall, like `NotifyIconSettings`; finding 11) and the new key if anything of it is
   left.
4. Write Part B's steps (below), set Status `blocked`, and stop. Leave nothing installed.

### Part B: Haik, about 5 minutes (the orchestrator relays)

Final steps (Part A, 2026-11-05). A worker runs the agent steps; Haik only looks and answers. The session must be
unlocked and the DELL screen on (the banner shows on the screen with the tray). No other agent may change power
settings meanwhile. `<S>` is the worker's scratch folder `<scratchpad>\p6-notice-app-icon\partB`.

Agent, before Haik:

1. Guard, read only: no lock `Local\Wakeshot.Tray`, no install traces (as `packaging/e2e.ps1` refuses: install
   folder, uninstall key, Run, RunOnce and switch values, Path entry, shortcut,
   `HKCU\Software\Classes\AppUserModelId\velopack.HaikAsatryan.Wakeshot`), no `%LocalAppData%\Wakeshot\journal.json`.
   Stop on any.
2. BEFORE, read only: the active plan; Balanced's lid AC and DC, sleep AC and DC and its `powercfg /qh` hash (at
   02:50: lid 1 and 1, sleep 18000 and 18000, hash `C64AFDB1CFDD13CF`); the values of Windows' own
   `HKCU\Software\Microsoft\Windows\CurrentVersion\Notifications\Settings\velopack.HaikAsatryan.Wakeshot` (at 02:30:
   `PeriodicNotificationCount` 14); whether `%LocalAppData%\Wakeshot` exists (it did not at 02:50).
3. A throwaway plan copy, made active: `powercfg /duplicatescheme 381b4222-f694-41f0-9685-ff5bb260df2e`, name it
   `Wakeshot test notice`, `/setactive` it. Why: the tray starts with Automatic on, sees the agents' own Claude Code
   sessions working and holds, so its writes must land on the copy.
4. Build release x64 of the tree into `<S>\target` (`-p wakeshot-tray -p wakeshot-cli --locked`, `RUSTFLAGS` unset),
   then `packaging/pack.ps1 -Version 0.1.0 -Cpu x64 -ExeDir <S>\target\x86_64-pc-windows-msvc\release -OutDir
   <S>\pack -Icon assets\wakeshot.ico`, then `<S>\pack\HaikAsatryan.Wakeshot-win-x64-Setup.exe --silent` at Medium
   (a `.cmd` opened with `explorer.exe`). Check: the key above has `DisplayName` "Wakeshot" and `IconUri`
   `%LocalAppData%\HaikAsatryan.Wakeshot\current\wakeshot-notice.png`, the file is there, no tray runs, no
   `%LocalAppData%\Wakeshot\config.json` (so the first start shows the first-run notice).
5. Start a screen grab loop: wait for a `wakeshot-tray.exe` process from the install folder, then grab the screen
   with the tray every 0.4 s for 10 s into `<S>\grabs-start-menu\`.

Haik:

6. Open Start, type `Wakeshot`, open it. A banner comes up at the bottom right within a few seconds. Say what the
   small picture left of "Wakeshot" in its header looks like: the violet square with the gold sun and a white line
   (as in the Start menu entry), a plain white square, or something else; and whether its colours look like the app
   icon's (a blue sun or an orange square means Windows swapped red and blue, as once seen in Phase 0).
7. Press Win+N and say whether a Wakeshot notice is listed there, then press Esc. (Before, Windows did not keep
   balloons there; the registered app id may change that.)

Agent:

8. Quit the tray with `taskkill /PID <pid>` (no `/F`; it quits in about 0.2 s) and check it ended. Delete
   `%LocalAppData%\Wakeshot\config.json` (this test install made it), so the next start shows the first-run notice
   again. Start the grab loop again into `<S>\grabs-direct\`, then start
   `%LocalAppData%\HaikAsatryan.Wakeshot\current\wakeshot-tray.exe` directly at Medium (`explorer.exe "<exe>"`), as
   the Run value does at sign-in.

Haik:

9. The same look at the second banner's header as in step 6.

Agent:

10. Quit the tray as in step 8. `%LocalAppData%\HaikAsatryan.Wakeshot\Update.exe --uninstall --silent` at Medium;
    wait up to 30 s for the install folder to go. Check gone: the app id key, the install folder, the shortcut, the
    Run value, the uninstall key, `%LocalAppData%\Wakeshot`. Read Windows' `Notifications\Settings` key again (it
    stays; note its values). Set Balanced active, delete the copy, and compare with BEFORE: the active plan, lid,
    sleep and the `/qh` hash. Delete `<S>\target`. Write the answers, the grab names and the reads into this card.
11. If a header still shows the white square: keep the grabs and the key's values as they were at that moment, and
    change no code; the orchestrator decides. The likely next test: Windows kept the default picture for this id on
    this laptop (14 balloons went under it before the key existed). A cheap check is a sign-out and sign-in by Haik,
    then steps 3 to 10 again (a card that signs out runs only when no other agent runs).

#### Haik's steps as sent, part 1 (steps 6 and 7; 2026-11-05 10:2x)

Before you start: the DELL dock is plugged in, the lid is open, you are signed in, and Do not disturb is off. Until
the orchestrator says the session is over, change no power setting in Settings or Control Panel (a test copy of your
plan is active, and such a change would write your own plan). Wakeshot is installed (a test install) and not started.
Your other Claude Code session may work during this part.

1. Open Start, type `Wakeshot`, open it. A banner comes up at the bottom right of the DELL screen within a few
   seconds. Say what the small picture left of "Wakeshot" in its header looks like: the violet square with the gold
   sun and a white line (as in the Start menu entry), a plain white square, or something else; and whether its
   colours look like the app icon's (a blue sun or an orange square means Windows swapped red and blue). If no
   banner comes, check that Do not disturb is off, and say so.
2. Press Win+N and say whether a Wakeshot notice is listed there, then press Esc.
3. Leave Wakeshot running. Send the orchestrator your answers to 1 and 2.

## Acceptance checks

- `cargo fmt` on own files; `cargo clippy -p` for each changed package (with and without `--all-targets`);
  `cargo test -p` on them; `e2e.ps1` passes on a local pack.
- Part B: both headers show the Wakeshot icon and name, by Haik's eye and the screen grabs.

## Out of scope

- The coffee cup in Settings, "Other system tray icons" (finding 4: a stale snapshot on this laptop only).
- `wakeshot-core`; `app/dispatch.rs`, `app/tray_mode.rs` (another card's).

## Questions for Haik

None. The design calls are below, as worker decisions for Haik's review.

## Worker decisions, for Haik's review

Taken while Haik slept (2026-11-05), each with its reason and when to change it.

1. **The hooks write the key, not the tray at each start.** Install and updated write it, uninstall removes it. Why:
   the hooks already own every other value Wakeshot leaves in the registry, with a log line each and the existing
   test seams; nothing is written at each start; the uninstall takes it away. Cost: the portable zip most likely runs
   no install hook (not verified), so it would keep the white square, as it would get no start at login and no
   `Path`. Change it when the portable zip or a
   Scoop package becomes a supported way to install, or a key goes missing on a user's PC: then the installed tray
   also writes it at start.
2. **The picture ships in the package**, `assets/wakeshot-notice.png` copied by `pack.ps1` to
   `current\wakeshot-notice.png`, not written by a hook. Why: Velopack puts it in place at each install, update and
   Setup over an install, and removes it with the app, so no hook file write can fail and nothing is left behind. A
   test keeps the file byte-equal to the art (`notify.rs` `notice_icon_file_matches_the_art`). Change it when the art
   changes often, or Windows keeps an old picture for the same path (then a file name with a version in it).
3. **48 by 48 px PNG.** Microsoft names no size; the shell's own header PNGs here are 28 and 56 px; the Windows App SDK
   uses the exe's small icon. 48 scales down by a whole factor at 100% (16 px) and 150% (24 px). Change it when the
   header looks soft at some scale.
4. **Only `DisplayName` and `IconUri`, both `REG_EXPAND_SZ`.** No `IconBackgroundColor`: neither the shell nor the
   Windows App SDK writes it, and a colour could fill the icon's clear corners. No `CustomActivator`: Wakeshot shows
   balloons, not toasts, and their clicks reach the tray's own window. Change it when Part B shows a plate or a wrong
   background behind the icon.
5. **No tray code for the app id.** `VelopackApp::run` already sets `velopack.HaikAsatryan.Wakeshot` for every start
   of an installed exe, from Start, the Run value or a restart after an update (velopack source; research "Notice
   header icon checks"). So the pick's second bullet holds already, and the guess that a Run start gets the shell's
   generated id was wrong for an installed exe. `velo.rs` got a comment that says so.
6. **A `--dry-run` started from an installed copy keeps the real app id.** Velopack sets it before the mode is read,
   so such a dry run shares the header and Windows' notification switch with the real app; it never writes the key.
   Left as is: dry runs run from `target\`, which has no explicit id; giving the dry run its own unregistered id may
   show no banner at all (an unregistered toast showed none, research "Toast"). Change it when a dry run from an
   install becomes a real use.
7. **The app id is a constant** (`notify::NOTICE_APP_ID`), not read from Velopack in the hook. Simpler, and
   `e2e.ps1` checks that the installed shortcut carries that id. Change it when `pack.ps1` names an AUMID or a
   Velopack update changes its rule.
8. **README "What stays"** now names `...\Notifications\Settings\velopack.HaikAsatryan.Wakeshot` instead of
   `...\Notifications\Settings\NotifyIconGeneratedAumid_<id>`: an installed tray's balloons go under the Velopack id,
   and the generated key was not made for it (`p6-int-haik-checks` finding 5). The new app id key is not listed
   there: the uninstall removes it.
9. **Part B: the worker quits the tray with `WM_CLOSE` posted to its `Wakeshot.Loop` window**, since `taskkill`
   without `/F` did not quit it (Found outside scope 6). Same message, the tray's own Quit path. Change it when
   taskkill works on the tray.
10. **Part B: Haik starts the second tray himself** with `<S>\start-direct.cmd` (`explorer.exe "<exe>"`, the start
    step 8 names), so he is at the screen when the banner shows; the relay takes minutes and a banner stays about
    5 s. Change it when Haik and the worker work at the same time.

## Found outside scope

Problems seen but not fixed.

1. **`README.md`, outside my lines, does not name the new key.** Lines 208 to 209 ("Registry, under ...") list the
   values Wakeshot keeps; proposed addition: "and `HKCU\Software\Classes\AppUserModelId\velopack.HaikAsatryan.Wakeshot`
   (the name and icon in the header of its notices)". Uninstall step 1 (line 466) could say "removes start at login
   (...), the sign-in restore entry, the `Path` entry and the notice name and icon first".
2. **`packaging/README.md` line 7** says `pack.ps1` copies `wakeshot-tray.exe` and `wakeshot.exe`; it now copies
   `assets/wakeshot-notice.png` too.
3. **`docs/phases/06-ship.md` "Hooks" table (about lines 56 to 59)**: the install, updated and uninstall rows do not
   name the notice app id step.
4. **`docs/architecture.md` section 5 "Notices" (about lines 1028 to 1031)** says Windows files a balloon under
   `NotifyIconGeneratedAumid_<id>` and that "uninstall has no stored balloon to clear". That holds for a tray run from
   `target\`; an installed tray's balloons go under `velopack.HaikAsatryan.Wakeshot` (section 6 now says so).
   Whether Windows keeps them in the notification centre now that the id is registered: Part B step 7.
5. **Commit `assets/wakeshot-notice.png` with the code.** `pack.ps1` now refuses to pack without it, so the release
   workflow would fail.
6. **Part B: `taskkill /PID <tray>` without `/F` does not quit the installed tray** (2026-11-05 10:29 elevated,
   10:30 at Medium; release x64 of the tree at 10:14). taskkill says "SUCCESS: Sent termination signal", the tray
   logs nothing and runs on. A `WM_CLOSE` posted to its `Wakeshot.Loop` window quits it in 138 ms with the full
   restore. The tray's top-level windows in `EnumWindows` order are `Wakeshot.TrayMenu`, `Wakeshot.TrayIcon`, then
   `Wakeshot.Loop`; `p5-menu-view` found the menu and icon windows ignore a sent `WM_CLOSE`. Likely cause: taskkill
   posts to only one window of the thread, the first (**not verified**). Docs that say otherwise:
   `docs/architecture.md` line 685, `docs/phases/06-integration.md` line 80, the comment in
   `crates/wakeshot-windows/src/tray/icon.rs` line 573, and this card's step 8 ("it quits in about 0.2 s"). Who is
   hit: scripts and users who end the tray with taskkill (it keeps holding). A fix to weigh: the menu and icon
   windows pass `WM_CLOSE` on to the loop as Quit; a test with real taskkill. No product decision.
7. **`docs/architecture.md` section 5 "Notices" and `docs/research.md` "Notice header icon checks"** can now say what
   Part B found: the header shows the app icon from both start paths, and a balloon is a banner-only toast that
   Win+N never keeps (`research.md` "Screen session of 2026-11-05"). The old line "Whether Windows keeps them in the
   notification centre now that the id is registered: Part B step 7" (item 4 above) is answered: no.

## Log

- 2026-11-05: card written by the orchestrator.
- 2026-11-05: worker started Part A. Haik asleep, so open calls are worker decisions for his review. Also fixing the
  old Startup switch bytes in `hooks.rs` tests (orchestrator's add-on).
- 2026-11-05 02:38: facts checked: Velopack's `VelopackApp::run` sets the explicit AUMID for every normal start of an
  installed exe (source), so the tray needs no call of its own; the Windows App SDK writes `DisplayName` and `IconUri`
  (`REG_EXPAND_SZ`) and no `IconBackgroundColor`; the Learn page is gone (404). Design: the hooks write the key,
  `pack.ps1` ships a 48 px PNG. `notify.rs`: `NoticeAppId` and its tests written.
- 2026-11-05 02:45: `hooks.rs`: install and updated register the app id, uninstall removes it (fake and real tests);
  the switch test bytes are now `01` + FILETIME and 12 zeros. `assets/wakeshot-notice.png` written by the new
  ignored writer test. `cargo test -p wakeshot-windows -- notify:: hooks::`: 38 passed, 1 ignored (the writer).
- 2026-11-05 02:50: `pack.ps1` ships the PNG; `e2e.ps1` checks the key, the icon file and the shortcut's app id at
  steps 4 to 6, the key gone at step 7, and refuses or tidies a leftover key. Clippy run stopped: `wakeshot-core`
  (the floor grace card's, mid-edit) does not compile at the moment; docs first, build again after.
- 2026-11-05 02:52: docs written (research "Notice header icon checks", architecture section 6, README "What stays").
  `wakeshot-core` compiles again. Clippy clean; tests pass; release x64 build; `e2e.ps1` at Medium 02:49:45 to
  02:50:16: 102 passed, 0 failed. Nothing installed after it; Haik's plan as before.
- 2026-11-05 02:58: Part B steps, decisions and Result written; build folder and packs deleted (logs kept in the
  scratch folder). Status blocked on Part B.
- 2026-11-05 09:5x: Part B started (worker, bundle with `p6-screen-reruns`, in that card's order). Orchestrator: no
  other keep-awake app, no Wakeshot installed or running, clang installed (no installer runs), Haik on the DELL dock
  only. `<S>` = `C:\Users\haika\AppData\Local\Temp\claude\f7c2c08a8f04f07b\845942db-47c9-4611-8481-99965de4381d\scratchpad\p6-notice-app-icon\partB`.
- 2026-11-05 10:11:06: step 1 guard passed (elevated, read only, `<S>\tools\state.ps1`): no lock
  `Local\Wakeshot.Tray`, no install trace (install folder, uninstall key, Run, RunOnce and switch values, Path entry,
  app id key, shortcut, Velopack temp folder, `HaikAsatryan.*` folders), no journal, no data folder. Step 2 BEFORE
  (`<S>\before.json`): Balanced the only plan and active; lid 1 and 1, sleep 18000 and 18000, energy saver DC 30,
  `/qh` hash `C64AFDB1CFDD13CF` (as at 02:50); Windows' `Notifications\Settings\velopack.HaikAsatryan.Wakeshot`:
  `PeriodicNotificationCount` 14, `PeriodicInteractionCount` 1, `LastNotificationAddedTime` 134383015833080771;
  `%LocalAppData%\Wakeshot` absent; no Wakeshot process.
- 2026-11-05 10:12 to 10:15: (bundle: the stale icon key deleted and the `%TEMP%` leftovers gone first, see
  `p6-screen-reruns`; no Visual Studio Installer runs.) Step 4 build: release x64 of the tree into `<S>\target`, exit 0
  in 116 s; tray 2,743,808 bytes (SHA256 `90FBEE166F6DDEE1...`), CLI 558,080 (`078B68AD661B6A0C...`); newest source
  `wakeshot-core` `scenarios/props.rs` 02:59:00. `pack.ps1` into `<S>\pack`: exit 0 in 6 s, Setup.exe 11,766,618
  bytes.
- 2026-11-05 10:15:29: steps 3 and 4 in one script at Medium (`<S>\tools\install.cmd` opened with `explorer.exe`;
  `install-medium.log`): copy `78a8cd32-8e80-47ba-9dea-6fb189411195` "Wakeshot test notice" made and active, then
  `Setup.exe --silent` exit 0 in 5.2 s; no process from the install folder 3 s later. The copy's `/qh` equals
  Balanced's but its name lines. Checks (elevated, 10:16): the app id key holds `DisplayName` "Wakeshot" and `IconUri`
  `C:\Users\haika\AppData\Local\HaikAsatryan.Wakeshot\current\wakeshot-notice.png`, both `ExpandString`; the PNG is
  there and byte-equal to `assets/wakeshot-notice.png`; installed tray hash equals the build; no tray runs; no
  `config.json` (data folder holds only the hook's log: install hook, Run value, Path, "notice app id ... registered
  with its icon", no journal).
- 2026-11-05 10:19:17: step 5 grab loop started, hidden, pid 23188 (`<S>\tools\grab.ps1`, primary screen = the DELL,
  3840 by 2160, per-monitor DPI aware; 15 s of grabs from the tray's start, not 10, for margin; waits up to 3 h) into
  `<S>\grabs-start-menu\`. A test grab at 10:10 gave a real picture (deleted).
- 2026-11-05 10:2x: Status blocked on Haik's part 1 (steps 6 and 7, below).
- 2026-11-05 10:3x: resumed with Haik's part 1 answers (relayed): step 6 "yes" (the violet square with the gold sun;
  no colour swap named); step 7 "no but it might be due to the fact that I have clicked notificaiton". Also: during
  the first start he opened Settings > Taskbar > Other system tray icons and Wakeshot was not in the list.
- 2026-11-05 10:24:25: the Start menu start, from the tray log and the grabs: tray pid 17944 started 10:24:25.610;
  "notice shown: FirstRun" 10:24:25.666; "notice click opened taskbar settings" 10:24:30.698 (Haik's click, 5 s
  later). Grab loop 23188: first grab 10:24:25.999, 37 grabs to 10:24:40.6, then it ended. `grab-04-102427-385.png`
  (crop `<S>\crop-start-menu-04.png`): the banner header shows the violet plate with the gold sun and the name
  "Wakeshot"; colours right; the body shows the large app icon and the first-run text. Then Keep awake on 10:26:08
  and off 10:26:16 (Haik), and Automatic held for Claude Code from 10:26:57 (the copy written, as designed).
- 2026-11-05 10:28: step 7 facts from Windows' own data. `Notifications\Settings\velopack.HaikAsatryan.Wakeshot`:
  `PeriodicNotificationCount` 14 to 15 and `LastNotificationAddedTime` 10:24:25.711 (the notice went under the
  registered id), `PeriodicInteractionCount` 1 to 2 (the click). A read-only copy of
  `%LocalAppData%\Microsoft\Windows\Notifications\wpndatabase.db` (with its WAL, into `<S>\wpn\`, read with
  Python `sqlite3`): handler 356 `velopack.HaikAsatryan.Wakeshot` holds 0 notifications now; every Wakeshot balloon
  payload in the file and the WAL, the first-run one too, is stored as `<toast bannerOnly="true">` with
  `ToastImageAndText02`. So Windows turns a balloon into a banner-only toast; that such a toast is never kept in the
  notification centre, with or without a click, is inferred from the attribute's name: **not verified** in Microsoft
  docs. Whether this notice was listed before the click: cannot be told from the data (a click removes it either way).
- 2026-11-05 10:29: step 8. `taskkill /PID 17944` (no `/F`), elevated at 10:29 and at Medium at 10:30:15: both said
  "SUCCESS: Sent termination signal", the tray logged nothing and ran on (checked for 10 s and 12 s). Its top-level
  windows, in `EnumWindows` order: `Wakeshot.TrayMenu`, `Wakeshot.TrayIcon`, `Wakeshot.Loop`, `MSCTFIME UI`, `IME`.
  A `WM_CLOSE` posted to the `Wakeshot.Loop` window at 10:30:53.6 quit it in 138 ms: restore of the copy (lid 0 to 1
  twice, sleep DC 0 to 18000), journal and RunOnce value gone, guard 20772 stopped, "Wakeshot ends with code 0"
  (Found outside scope 6). The copy's `/qh` then equal to Balanced's but the name lines; Balanced `C64AFDB1CFDD13CF`.
  `config.json` (`{"version": 1, "automatic": true}`, kept as `<S>\config-first-start.json`) deleted 10:31:08. Grab
  loop for the direct start: pid 35404 from 10:31:25 into `<S>\grabs-direct\`. Haik starts the tray himself with
  `<S>\start-direct.cmd` (`p6-screen-reruns` decision 2).
- 2026-11-05 10:3x: Status blocked on Haik's part 2 (step 9 is its section A; the text is in `p6-screen-reruns`).
- 2026-11-05 10:50: step 9 (part 2 section A), Haik's words: "1. Not sure what color are you speaking about as in
  tray it's not colorful, just white. And not sure what is header. ... 2. Win+N case is again not working.
  Notification comes but not persist in history of windows notifications." So he looked at the tray glyph, not the
  banner header; the header is taken from the grabs (orchestrator's call). Direct start: tray pid 34172 started
  10:45:33.428 by `start-direct.cmd`; "notice shown: FirstRun" 10:45:33.484; no click logged. Grab loop 35404: 38
  grabs 10:45:33.7 to 10:45:48.9. `grab-05-104535-568.png` (crop `<S>\crop-direct-05.png`): the header shows the
  violet plate with the gold sun and "Wakeshot", the same as from Start. Zoomed side by side (`<S>\header-icons-x8.png`,
  40 px crops at 3288,1838 of both grabs): the two header pictures are the same, about 22 px, plate pixel
  `#7250F3` (brand plate `#7957F4` to `#6B49F3`), gold sun, light line; no red and blue swap. Windows'
  `Notifications\Settings` key: count 16, last added 10:45:33.522, interactions still 2 (no click). Not kept in Win+N
  with no click either, so the click of part 1 was not the reason: a balloon is a banner-only toast (10:28 entry).
- 2026-11-05 11:29:58 to 11:31: step 10 (after the reruns of `p6-screen-reruns`). Tray log and System log saved to
  `<S>\partB-wakeshot.log` and `<S>\partB-system-power.txt` first. Quit: `WM_CLOSE` posted to the `Wakeshot.Loop`
  window (decision 9; the same window order as at 10:29): restore of the copy, journal and RunOnce value gone, guard
  stopped, exit 0, 159 ms. `Update.exe --uninstall --silent` at Medium (`<S>\tools\uninstall.cmd` via `explorer.exe`)
  11:30:17: exit 0 in 4.3 s, install folder gone 4.4 s after the start. `powercfg /setactive` Balanced and `/delete`
  the copy: exit 0 each. AFTER (`<S>\after.json`, 11:30:32) against BEFORE: the same active plan (Balanced, the only
  plan), lid 1 and 1, sleep 18000 and 18000, energy saver DC 30, `/qh` `C64AFDB1CFDD13CF`, Run, RunOnce,
  StartupApproved and Path; no install trace (the app id key, the install folder, the shortcut, the Run value, the
  uninstall key, the Path entry all gone), no `%LocalAppData%\Wakeshot`, no journal, no Wakeshot process. Windows'
  `Notifications\Settings\velopack.HaikAsatryan.Wakeshot` stays: `PeriodicNotificationCount` 19,
  `PeriodicInteractionCount` 2, last added 11:24:39.069. `<S>\target` and `<S>\pack` deleted 11:30:48. Status done.

## Result

**Done 2026-11-05.** Part A done at 02:58; Part B done 10:11 to 11:31, bundled with `p6-screen-reruns`.

Part B:

| Step | Outcome |
|---|---|
| 1 guard, 2 BEFORE | Passed; Balanced lid 1 and 1, sleep 18000 and 18000, `/qh` `C64AFDB1CFDD13CF`; Windows' notification count 14 |
| 3 to 5 | Copy "Wakeshot test notice" active; release x64 built, packed, installed at Medium; app id key and PNG right; no tray, no `config.json`; grab loop |
| 6 Start menu start | **Pass.** Haik: "yes". Grab: violet plate, gold sun, "Wakeshot" in the header, colours right |
| 7 Win+N | No Wakeshot notice. Windows stores each balloon as a banner-only toast (`bannerOnly="true"`), so it is not kept, with or without a click (inferred from the attribute; Haik clicked this one) |
| 8 quit, second start | `taskkill` without `/F` did not quit the tray (Found outside scope 6); a `WM_CLOSE` to its loop window did. Haik started the exe himself (decision 10) |
| 9 direct start | **Pass by the grabs** (Haik looked at the tray glyph): the same header picture and name as step 6. Not in Win+N either, with no click |
| 10 teardown | Uninstall at Medium exit 0; Balanced active, copy deleted; AFTER equals BEFORE; build folder and packs deleted |
| 11 | Not needed: no white square |

Acceptance check, Part B: both headers show the Wakeshot icon and name: pass, step 6 by Haik's eye and the grabs,
step 9 by the grabs (the orchestrator's call, 2026-11-05). Part A's checks: see below.

Open items for the orchestrator: Found outside scope 1 to 5 (from Part A), 6 (taskkill does not quit the tray: a fix
card; it also makes `docs/architecture.md` line 685, `docs/phases/06-integration.md` line 80 and the `icon.rs` line
573 comment wrong as they stand) and 7 below. The Win+N result means notices stay banner only; whether Haik wants
them kept in the notification centre would be a product question (it would take toasts, not balloons); my default:
leave as is.

Part A (2026-11-05 02:58), kept for the record:

What changed:

- The install and updated hooks write `HKCU\Software\Classes\AppUserModelId\velopack.HaikAsatryan.Wakeshot`
  (`DisplayName` "Wakeshot", `IconUri` naming `current\wakeshot-notice.png`, both `REG_EXPAND_SZ`); the uninstall
  removes it. The id is the one Velopack already gives every start of an installed tray, so no tray code was needed
  (decision 5). The package now ships a 48 px PNG of the app icon beside the tray exe.
- The `hooks.rs` test switch bytes are now the real forms: `01` and the FILETIME for off, 12 zero bytes for on.

Files: `crates/wakeshot-windows/src/notify.rs` (`NoticeAppId`, the constants, tests: scratch key write and remove,
inner NUL, the real key path, PNG equal to the art, an ignored writer), `crates/wakeshot-windows/src/hooks.rs`
(`register_notices`, `remove_notices`, `Registered`, `Places.notices`, fake and real tests, the switch bytes),
`crates/wakeshot-tray/src/velo.rs` (a comment), `assets/wakeshot-notice.png` (new, 1,886 bytes), `packaging/pack.ps1`,
`packaging/e2e.ps1` (14 new checks), `docs/architecture.md` section 6, `README.md` "What stays", `docs/research.md`
"Notice header icon checks", this card. No module list, `Cargo.toml` or `Cargo.lock` changed.

Checks:

- `rustfmt --check` on the three Rust files and `cargo fmt -p wakeshot-windows -p wakeshot-tray -- --check`: clean.
- `cargo clippy -p wakeshot-windows -p wakeshot-tray --locked --target x86_64-pc-windows-msvc -- -D warnings`, with
  and without `--all-targets`: clean. ARM64 not run (nothing CPU-specific changed).
- `cargo test --locked -p wakeshot-windows -p wakeshot-tray`: `wakeshot-windows` 585 passed, 24 ignored; tray 8
  passed; `recovery` 5 passed, 2 ignored; the e2e suites ignored as designed.
- `cargo build --release --locked --target x86_64-pc-windows-msvc -p wakeshot-tray -p wakeshot-cli`: done.
- `packaging/e2e.ps1` at Medium on a local pack: 102 passed, 0 failed, exit 0; the guard passed first; nothing left
  installed; the real plan as before.

Open: Part B (both headers by Haik's eye and screen grabs; also whether Win+N now lists the notices). Found outside
scope 1 to 5 for the orchestrator (stale lines in files I do not own; commit the new PNG with the code).
