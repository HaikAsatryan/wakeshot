# Show the Wakeshot icon in the notice header

Phase: 6 (tray fix). Status: ready. Needs Haik: Part B only, about 5 minutes at the screen (he looks at two notice
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

Draft; Part A fills in paths. The agent installs a local pack and starts the tray from the Start shortcut; Haik
watches the first-run balloon's header and says what icon it shows. Then the agent quits it, starts the exe directly
(as the Run value would), shows a balloon (for example Keep awake from the menu, then Off) and Haik says again. The
agent takes a screen grab of each, then uninstalls and checks the keys are gone.

## Acceptance checks

- `cargo fmt` on own files; `cargo clippy -p` for each changed package (with and without `--all-targets`);
  `cargo test -p` on them; `e2e.ps1` passes on a local pack.
- Part B: both headers show the Wakeshot icon and name, by Haik's eye and the screen grabs.

## Out of scope

- The coffee cup in Settings, "Other system tray icons" (finding 4: a stale snapshot on this laptop only).
- `wakeshot-core`; `app/dispatch.rs`, `app/tray_mode.rs` (another card's).

## Questions for Haik

None.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-05: card written by the orchestrator.

## Result

What was done, files changed, checks and outcomes, open items.
