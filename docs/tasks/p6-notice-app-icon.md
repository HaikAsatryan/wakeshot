# Show the Wakeshot icon in the notice header

Phase: 6 (tray fix). Status: blocked (Part A done 2026-11-05; Part B needs Haik at the screen). Needs Haik: Part B only, about 5 minutes at the screen (he looks at two notice
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

## Result

Part A done 2026-11-05; Part B (Haik at the screen) is open.

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
