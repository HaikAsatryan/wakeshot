# Short check: does a silent winget upgrade start Wakeshot again

Phase: 6. Status: done (new 2026-11-07). Needs Haik: no (Haik, 2026-11-07: "Ok if it's only by name upgrade but
not the --all case, then ok, but you can do short research if there is a fix. No need too much effort.").
Agent: `wakeshot-worker`. Size: about 40K tokens; keep it short.
Runs after: none. Parallel with: every card; no cargo build in this card.
Build folder: none.

## Goal

Today the README says a silent `winget upgrade HaikAsatryan.Wakeshot` leaves Wakeshot closed until the next sign-in
or a start by hand. Find out, with current sources, whether a small and clean fix exists: for example a Velopack
install or update hook that starts the tray again when it ran before the upgrade.

## Read first

- README, the part that names the winget upgrade limit.
- `docs/research/launch.md` (winget rules for an app that updates itself), `docs/tasks/p6-velopack-facts.md` Result,
  `docs/tasks/p6-hooks.md` Result, `crates/wakeshot-windows/src/hooks.rs` and `crates/wakeshot-tray/src/velo.rs`
  (read only).
- Velopack's current docs and source for the hooks Setup.exe runs over an existing install, and how winget runs the
  installer (silent switches).

## Deliverables

1. In the Result: what happens today on a by-name upgrade and on `winget upgrade --all`, each with its source; the
   fix if one is small and clean (which hook, what it would do, how it knows the tray ran before, what could go
   wrong, about how many lines, which files); or why there is none.
2. A short line in `docs/research.md` (additions only; re-read right before the edit) with the source.
3. No code change in this card. If a fix is worth building, the orchestrator schedules it after the Rust cards that
   run now.

## Rules

- Never run git, not even read-only. No installs, no change to Haik's Wakeshot or his settings. Never read token files.
- Plain short English, no em dashes, no emojis. Write Log lines as you go.

## Questions for Haik

None tonight: decide and log "for Haik's review".

## Found outside scope

## Log

- 2026-11-07 03:50: card written by the orchestrator (session wakeshot-4e).
- 2026-11-07 03:47 (laptop clock): worker started; reading README, launch.md, velopack and hooks cards, hooks.rs,
  velo.rs.
- 2026-11-07 about 03:49: read Velopack 1.2.161 (still the latest release) `install.rs`, `setup.rs`, `util_windows.rs`
  and winget-cli main `ShellExecuteInstallerHandler.cpp`, `UpdateFlow.cpp`, `PackageTrackingCatalog.cpp` via
  `gh api`. Setup has no start-after-silent option; winget passes our `--silent` unless `-i`. Next: how a hook
  could know the tray ran, and whether Velopack's hook runner kills children.
- 2026-11-07 about 03:51: Velopack starts hooks with plain `CreateProcess` and no job object (`process_win.rs`), and on
  timeout kills only the hook process (`windows/util.rs` `run_hook`), so a child the hook starts lives on. winget
  waits only on the Setup process handle, no job. `state.json` already names the tray (`pid`, `procStart`) but
  nothing tells a Quit from a kill. First update check waits 5 min (`update.rs`), so a tray started from the hook
  never races Setup's lock. Writing Result.
- 2026-11-07 03:53: Result written; research line added; Status done.

## Acceptance

- Result names what a by-name upgrade and `--all` do today, each with a source: done.
- Fix or why none, with hook, signal, risks, size and files: done.
- One line in `docs/research.md`, additions only, re-read right before the edit: done.
- No code change, no git, no installs, no cargo build: kept.

## Result

Status: done 2026-11-07. Sources read with `gh api` on 2026-11-07: Velopack tag `1.2.161` (our pin, still the latest
release, 2026-09-29) and winget-cli `master` (latest release v1.29.380). Not tested on this laptop.

### What happens today

- **By name** (`winget upgrade HaikAsatryan.Wakeshot`): winget passes our `SilentWithProgress` switch, `--silent`
  (`ShellExecuteInstallerHandler.cpp`, `GetInstallerArgsTemplate`; our manifest maps both silent switches to
  `--silent`). Setup over an install kills every process from the install folder (`force_stop_package`), replaces
  the folder, runs only the install hook (which restores power settings when the journal's owner is gone), writes the
  uninstall entry, and starts the app only when not silent (`install.rs`, end of `install_impl`). Setup has no
  option to start the app after a silent install (`setup.rs` args). So the tray stays off until sign-in. The README
  line is right. Only `winget upgrade -i` passes no switch: Setup then shows its overwrite dialog and starts the app.
- **`--all`**: when winget installed Wakeshot, `RequireExplicitUpgrade` stores `PinnedByManifest`
  (`PackageTrackingCatalog.cpp`) and `--all` skips it (`UpdateFlow.cpp`: "Skipping ... as it requires explicit
  upgrade"). A GitHub install has no such mark, so `--all` upgrades it exactly as above (same source comment: "the
  downside of not working with packages installed from another source"). Matches `docs/research/launch.md`.
- **How often it bites**: Setup runs only when winget's catalog is ahead of the installed version. A running tray
  takes our own updates within about 12 hours, but defers the apply while it holds, so the gap is widest exactly
  during long agent work. Inferred, not measured.

### Is there a small clean fix: yes

The install hook can start the tray again. Facts it relies on (source): Velopack starts hooks with plain
`CreateProcess`, no job object (`process_win.rs`), and on timeout kills only the hook process (`windows/util.rs`
`run_hook`); winget waits only on the Setup process handle. So a detached child of the hook lives on. The tray's
first update check waits 5 minutes (`update.rs` `FIRST_CHECK_MS`), so it never races Setup's lock. A Setup that fails
after the hook kills the folder's processes again in its rollback (`install.rs`), which is today's result.

How the hook knows the tray ran: `state.json` already names the tray (`pid`, `procStart`), but a Quit and a kill
look the same. Pick, with one small tray change:

| Signal | Tray change | Wrong when |
|---|---|---|
| Pick: `state.json` names a process that is now gone, and no Quit mark | on menu Quit write a mark file; delete it at start | a tray that crashed earlier comes back (harmless: idle tray changes nothing) |
| Run value was set before this install (read before `set_run`) | none | a tray the user quit comes back; a hand-started tray with autostart off does not |

What the fix does: `on_install` reads the signal before `set_run`; after the restore, when it says the tray ran and
the hook is not elevated, it starts `current\wakeshot-tray.exe` detached, no inherited handles, working folder
`current`, no arguments. The new tray's own start does the rest (single instance `Local\Wakeshot.Tray`, journal,
holds from `holds\` and the session scans), so a hold an agent had comes back within seconds.

What could go wrong:

1. Elevated Setup (winget from an admin terminal; agents on this laptop run elevated): starting would give an
   elevated tray. The fix skips the start then and logs one line, so that case stays as today. Change it when
   elevated upgrades prove common: start through Explorer's shell (unelevated), about 50 more lines.
2. `winget upgrade -i` or a hand-run Setup also starts the app itself: two starts, the single-instance mutex ends the
   second. Not tested.
3. A user who crashed or killed the tray long ago gets it back on the next upgrade. Accepted.

Size: about 70 lines of code and 60 of tests, plus one e2e step (Setup `--silent` over a running install, then the
tray is running again). Files: `crates/wakeshot-windows/src/hooks.rs` (two `HookEnv` methods, the real ones, the
`on_install` change), `crates/wakeshot-windows/src/store.rs` (one `DataFile` name for the mark), the tray's Quit and
start path in `crates/wakeshot-windows/src/app/` (window or startup, whichever owns Quit), `packaging/e2e.ps1`, and
the README limit text.

### Call, for Haik's review

Build it as one small card after the Rust cards that run now, with the pick above. Reason: it is the one path where
the user's own explicit act silently drops a running agent's hold until sign-in, which breaks the core promise, and
the fix is small, local to the install hook, and falls back to today's behaviour on every failure. Change it when
the card's estimate grows past about 200 lines: then keep the README limit instead. Until it ships, the README line
stays as it is.

### Files changed

- `docs/tasks/p6-winget-upgrade-restart.md` (this card).
- `docs/research.md`: new section "winget upgrade over a running Wakeshot (2026-11-07)", additions only.

### Open items

- The fix card, if the orchestrator schedules it. Two "not tested" points above go into its e2e step.
