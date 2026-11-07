# Logs that clean themselves and a bug report users can send

Phase: 6. Status: done (new 2026-11-07). Needs Haik: no (Haik, 2026-11-07: follow the latest best practices; no
questions to him).
Agent: `wakeshot-worker`. Size: about 150K tokens.
Runs after: `p4-idle-cpu`. Runs alone among Rust cards if it changes a contract (a menu item or CLI command is in
`architecture.md` section 2); otherwise beside non-Rust cards.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-logs-diagnostics\target`; delete it when done.

## Goal

Haik, 2026-11-07: "do we autocleanup log at some point or not? Do we have proper logs at appropriate way that real
users can submit logs for any bugs and we confidently find any problems? If any answer is no then seperate card for
that to to follow all the latest best practices."

Today (checked 2026-11-07): the log rolls only when the tray starts (`startup.rs` calls `LogWriter::roll`): over
1 MB it keeps three files. A tray that runs for weeks never rolls, so the file grows until the next sign-in. Users
send logs by hand: "Open log" or `wakeshot log`, then copy lines into the bug form. The install check found no log
line per Claude Code session (its holds were read from `state.json`).

## Deliverables

1. Research first, with current sources: how well-made Windows tray apps and CLIs keep logs and take bug reports
   (size and age caps, rolling while running, a diagnostics bundle, what goes in it, privacy). Short notes in
   `docs/research.md` with sources.
2. Clean-up: the log rolls by size while running too, not only at start; a cap on the total size of the logs folder;
   old files go. Every writer process (tray, guard, CLI, hook) stays safe (the four-writer rule in `architecture.md`
   section 8). Tests for each rule.
3. Enough to find a problem from the log alone: a header at each start (Wakeshot version, Windows version and build,
   CPU, Modern Standby or S3, power plan GUID, the lid and sleep values Wakeshot reads, which sources run); one line
   per hold start and end per agent and session (a short id, never a prompt, title or full path), with the reason;
   every source's format problem; every restore. Check each agent source and the power layer for silent decisions.
4. A way to send it: a menu item or CLI command (for example `wakeshot diagnostics`) that writes one zip or text
   bundle (the logs, the state and config files Wakeshot owns, the header facts) to a place the user picks or the
   Desktop, and opens Explorer on it. It holds no prompt text, no token, no other app's files, and replaces the user
   name in paths. A test proves the bundle never holds a planted prompt or token string.
5. `.github/ISSUE_TEMPLATE/bug-report.yml` and the README "Log" or "Reporting a bug" part: how to make and attach the
   bundle. `docs/architecture.md` section 8 "Log" updated.
6. A contract change (a new menu command or CLI command) only in a run alone; say so in the Log.

## Acceptance checks

- Tests for rolling while running, the folder cap, the header, the per-session lines and the bundle's privacy; each
  fails before its change.
- Whole-workspace checks on x64 and ARM64 (`CLAUDE.md`, Checks).

## Rules

- Never run git, not even read-only. Never read token files. Prompt text is never logged. Kill only processes you
  started, by PID. Never set `RUSTFLAGS`; cargo may not be on PATH: `%USERPROFILE%\.cargo\bin\cargo.exe`; ARM64
  needs the Build Tools `VC\Tools\Llvm\x64\bin` folder on that cargo command's PATH only.
- Plain short English, no em dashes, no emojis. Write Log lines as you go.

## Questions for Haik

None: Haik asked for no questions; decide by best practice and log each call "for Haik's review".

## Found outside scope

From the read-only audit of 2026-11-07 04:28 (an Explore agent; file and line as it reported them, not re-checked
line by line by this card). Each leaves no log line for a decision a bug report may need. Not fixed: watch and power
code beyond this card's two small core lines.

1. Claude Code and Copilot report a transcript problem only while the session works (`sources/claude/mod.rs`
   259-266, `copilot.rs` 856-859), so an unreadable cron or wakeup result in an idle session never holds and says
   nothing. By design today; worth a look.
2. Liveness answers Gone without a line when no process with the agent's image holds the lock or matches
   (`watch/agents.rs` `any_process` 555-569, `image_since` 647-648). An agent that renames its exe would never hold,
   silently; the new count lines at least show its sessions are seen.
3. OpenCode: `no_sqlite` (`sqlite.rs` 220-222), a database without `session_v2` (`opencode.rs` 178-183) and a missing
   `-wal` (`watch/agents.rs` 1456-1460) give no line.
4. An unreadable folder counts as empty in `FindTail` and `FindNewest` (`watch/read.rs` 246-248, 276-278), with no
   `Unlisted` problem: a Codex rollout or a Claude transcript is then never found, silently.
5. Codex skips a non-JSON `event_msg` line and any unknown turn event type without a problem (`codex.rs` 274-276,
   298): a renamed `task_started` would mean "never holds", silently.
6. A Gone hold file's delete is logged only when it fails (`app/io_thread.rs` 360-362).
7. Power safety paths (for the power safety reviewer, not changed here): a restore drops an entry whose plan is gone
   or whose value is no longer ours, and an apply skips keys, and the log gives counts or written keys only, not why
   (`ownership/restore.rs` 143, 151; `ownership/apply.rs` 318-323; `power/journal.rs` `log_restored`). A journal save
   that fails after a good apply is swallowed (`ownership/apply.rs` 211-216, `power/journal.rs` 501), and two
   `let _ =` results in `ownership/apply.rs` 380-387 (RunOnce clear, guard stop).
8. `watch/thread.rs` 328: `let _ = self.watch.add(...)` for the data folder and `holds\`.
9. `docs/phases/01-design-skeleton.md` line 100 and `docs/phases/03-windows-power.md` line 106 still describe the old
   roll (1 MB, three files at start). Phase files are history; the orchestrator decides.
10. `docs/product.md` line 248 says "Everything the CLI controls, the menu controls too (except `wakeshot run`)", and
    its CLI list (line 250) does not name `wakeshot diagnostics`. Not this card's file. `diagnostics` controls nothing
    (it only reads and writes one file), so my default is to add it to that list as a CLI-only support command, the
    way `run` is an exception; a menu item is the other way (see Result).

## Log

- 2026-11-07 02:43: card written by the orchestrator.
- 2026-11-07 03:47: worker started (session wakeshot-4e). Runs alone among Rust cards (orchestrator, 03:50), so a
  contract change (menu item, CLI command) and a `Cargo.toml`/`Cargo.lock` change are allowed in this run.
- 2026-11-07 03:58: read the log module, startup, tray mode, core hold logging, CLI, store. Haik's installed log:
  250 KB after about 2 days (one 1 MiB file is about a week). Research done (PowerToys bug report tool, OWASP logging
  cheat sheet, Serilog file sink defaults, GitHub attachment limits, `SHOpenFolderAndSelectItems`,
  `SYSTEM_POWER_CAPABILITIES`); notes go to `docs/research.md` at the end.
- 2026-11-07 04:00: design calls (each for Haik's review, reasons in Result): every line opens, writes and closes the
  file, so no writer is stranded by a roll; any writer rolls when the file is over 1 MiB, to a UTC-stamped name, no
  lock; prune keeps the folder at 8 MiB and drops rolled files older than 30 days; a CLI command
  `wakeshot diagnostics [folder]` writes one text file (no menu item); per-session hold lines in the core with a short
  id (a pid as is, any other id as an 8-hex hash).
- 2026-11-07 04:06: `rolls_while_running` written first and run on the old code: failed (no rolled file after 1.1 MB
  of lines). Log module rebuilt: per-line open, roll by handle (size check and rename on one handle, no lock),
  `wakeshot.<UTC>.log` names, prune by age and folder size, `log_files` for the bundle. 13 log tests pass, among them
  the four-writer test, now also with a roll every 64 KiB (32 files, every line whole, once, in order).
- 2026-11-07 04:12: per-session and per-hold-file lines in the core (`core/holds.rs`, `state.rs`, `core.rs`). The
  summing-up lines ("Holding starts: ...", "Hold ends: ...") stay word for word: `media/scripts/measure-speed.ps1` and
  past cards read them. `each_session_gets_its_own_start_and_end_line` and `busy_session_holds_and_ends` fail with
  the new lines turned off (checked, then restored) and pass with them; core: 738 lib tests and every integration
  test pass.
- 2026-11-07 04:19: new `wakeshot-windows::diagnostics` (machine facts, `Redact`, `write_bundle`, `desktop`),
  `power::plan::active_values`, `shell::show_in_folder` (windows feature `Win32_UI_Shell_Common`, no `Cargo.lock`
  change), the tray's start header (`tray_mode::log_start_header`, area `start`), and the CLI command
  `wakeshot diagnostics [folder]` (`args`, `command`, `main`, new `diagnostics.rs`). The privacy test plants a prompt
  and two tokens in agents' files, stray files, a hold reason, a log line and behind two junctions: none reaches the
  bundle, and the planted user name never shows. Diagnostics, CLI and header tests pass.
- 2026-11-07 04:25: docs: `architecture.md` (section 2 module rows, section 4 CLI row and paragraph, section 8 "Log"
  rewritten), README (the log bullet, a bug report bullet, the `diagnostics` line in the help block and its bullet),
  `bug-report.yml` (asks for the diagnostics file first), `SECURITY.md` (the diagnostics file in the private-data
  line), `docs/research.md` (new section "Logs that clean themselves and a bug report bundle"). An Explore agent
  audits the sources, watch and power code for silent decisions, read only.
- 2026-11-07 04:28: audit back (10 gaps ranked). Fixed here, both small and in the core: a line with each source's
  session count when it changes ("Codex: 1 session in its files"), so a session that never holds is still seen; and
  a line for every failed power request change, also a failed release, which had no trace at all. The rest go to
  "Found outside scope" (watch and power code, two of them reviewed power-safety paths). Log times before this line
  were first written ahead of the clock and corrected from test output times.
- 2026-11-07 04:31: first whole-workspace run (`checks-ws.ps1`): fmt, x64 clippy both ways and `cargo test
  --workspace --locked` pass; release and ARM64 steps run. Install state before the update read: tray 41472 and
  guard 564 at Medium; installed exes `D05EC7D2...` and `C0E7D90E...`; `config.json` `4EBE0C4B...`; lid AC 0, DC 0,
  sleep AC 18000, DC 0 and one SYSTEM request: the tray's held values for the orchestrator's session.
- 2026-11-07 04:35: whole-workspace checks all pass (fmt; x64 clippy with and without `--all-targets`; `cargo test
  --workspace --locked`; x64 and ARM64 release builds; ARM64 clippy both ways; no `warning` or `error` line; no CRT
  import). x64 `wakeshot-tray.exe` 3,179,520 bytes `9DEAB8A4...` (was 3,155,968), `wakeshot.exe` 873,472 bytes
  `8BB76101...` (was 815,616); ARM64 2,797,056 `38B303D6...` and 834,048 `3E979ACA...`. A bundle of the real data
  folder (ignored test `real_bundle_of_this_pc`, output in the scratchpad, no Explorer): 255,555 bytes, 7 files, no
  whole-word or `Users\` user name left, 79 `%USERPROFILE%`. x64 packed: Setup.exe 12,532,026 bytes `9048DA11...`.
- 2026-11-07 04:36:52: Haik's tray 41472 quit by `WM_CLOSE` to its `Wakeshot.Loop` window: exit 0 after 73 ms, guard
  564 exit 0; its log wrote lidAc 0 to 1, lidDc 0 to 1, sleepDc 0 to 18000, journal deleted. Read 04:36:55: lid AC 1,
  DC 1, sleep AC 18000, DC 18000, no SYSTEM request: Haik's own values.
- 2026-11-07 04:37:05 to 04:37:08: `Setup.exe --silent` at Medium through `explorer.exe` (S-1-16-8192), exit 0; "User
  chose to overwrite existing installation", install hook ran, "Installation completed successfully"; no tray
  started. Installed `wakeshot-tray.exe` `9DEAB8A4...` and `wakeshot.exe` `8BB76101...` equal the build;
  `config.json` same bytes and time.
- 2026-11-07 04:37:24: tray started through `explorer.exe`: tray 20124 and guard 50240, both Medium, parent
  explorer.exe. The log has the new header (eight `start:` lines, lid Sleep and sleep 18000 s read before its apply),
  then "Claude Code: 2 sessions in its files", "Claude Code session 15240 holds: working" (pid 15240 is the
  orchestrator's `claude`, started 02:24), "Holding starts: Claude Code", "Copilot: 7 sessions in its files". Read
  04:37:39: lid AC 0, DC 0, sleep AC 18000, DC 0, one SYSTEM request: the same as before the update.
- 2026-11-07 04:37:53: the installed `wakeshot.exe diagnostics <scratchpad>\diag-live` at Medium through Explorer:
  exit 0, 262,587 bytes, "Elevated: no", "Tray: running (pid 20124)", no whole-word user name; no "Explorer did not
  open" line, so `SHOpenFolderAndSelectItems` succeeded. It opened a File Explorer window or tab on that folder on
  Haik's screen; left open, since a tab cannot be told from a window by its title and closing the window could close
  his own tabs.
- 2026-11-07 04:39: final tree rechecked (fmt, CLI tests 107 passed); build folder (`target\`), both bundles and the
  draft files deleted. Kept in `<scratchpad>\p6-logs-diagnostics\`: `pack-x64\`, the scripts, `logs-ws\`, `run\`, the
  power and state reads. No process of mine runs.

## Result

Status: done (2026-11-07 04:40). Ran alone among Rust cards; it adds one CLI command (`diagnostics`) and one windows
feature (`Win32_UI_Shell_Common`); `Cargo.lock` unchanged.

**What changed for users**

- The log cleans itself: any process (tray, guard, restore, CLI, hook) rolls `wakeshot.log` once it passes 1 MiB, also
  while the tray runs for weeks. Rolled files are `wakeshot.<UTC time>.log`; those older than 30 days go, and the
  oldest go while all logs take more than 8 MiB. The old `wakeshot.1.log` and `wakeshot.2.log` are cleaned the same
  way.
- Each tray start logs a header: Wakeshot version and build, Windows name, edition, release and build, processor,
  PC model and firmware, Modern Standby or S3, hibernate, lid, battery, AC or battery, the active plan with every
  value Wakeshot reads, and whether it runs elevated.
- New log lines: one per agent session at each start and end of its hold, with a short id (a pid as is, else an
  8-hex hash) and the reason; the same per hold file; each source's session count when it changes; every failed
  power request change (a failed release had no trace).
- `wakeshot diagnostics [folder]` saves one text file on the Desktop (or the folder given) and shows it in Explorer:
  the machine facts, whether the tray runs, and Wakeshot's own files (settings, state, journal, hold files, hook
  files, logs), found by name only, links skipped. The profile folder shows as `%USERPROFILE%`, other user names in
  paths as `<user>`, token-like words as `[token removed]`. The bug form asks for this file first.

**Decisions for Haik's review** (each reversible; reason, then when to change it)

1. A CLI command, no tray menu item. The menu and the README's menu shots stay as approved; Wakeshot's users run
   agents in terminals. PowerToys puts "Report Bug" in its tray menu. Change it when a user without a terminal needs
   to report a bug. `docs/product.md` line 248 says the menu controls all the CLI controls (Found outside scope 10).
2. One plain text file, not a zip: the user can read what they send, no new crate, GitHub takes `.txt` up to 25 MB.
   Change it when the bundle needs a binary file.
3. Caps: roll at 1 MiB (as before), 8 MiB for all logs, 30 days. Haik's log grows about 125 KB a day, so that is about
   a month. Change it when 8 MiB holds less than two weeks for real users.
4. Every line opens, writes and closes the file, so a roll by any process strands no writer; the roll checks the size
   and renames on one handle, so no lock is needed. Cost: one open per line; the idle tray writes almost none. Change
   it when the idle CPU measure shows the opens.
5. Session ids in the log: a pid as is, anything else as an 8-hex FNV-1a hash (OWASP: hash logged session ids).
   Change it when a bug needs the agent's own id.
6. The header adds the PC model, firmware and elevation, which the card did not list: sleep bugs often depend on
   them. None names the user.
7. The token filter is a second line of defense: Wakeshot's files hold no tokens by design. It catches known prefixes
   (`sk-`, `ghp_`, `AKIA`, `eyJ`, ...) and words of 32 or more letters and digits that are not GUIDs.
8. README: besides the log bullet, I added a bug report bullet next to it and the `diagnostics` line to the Commands
   help block and its bullet list, so the README matches `wakeshot --help`. Change nothing else there.

**Files**

- Changed: `crates/wakeshot-windows/src/log.rs`, `crates/wakeshot-windows/src/lib.rs`,
  `crates/wakeshot-windows/src/shell.rs`, `crates/wakeshot-windows/src/power/plan.rs`,
  `crates/wakeshot-windows/src/app/tray_mode.rs`, `crates/wakeshot-windows/src/app/startup.rs`,
  `crates/wakeshot-windows/Cargo.toml`, `crates/wakeshot-core/src/core.rs`, `crates/wakeshot-core/src/core/holds.rs`,
  `crates/wakeshot-core/src/state.rs`, `crates/wakeshot-core/src/scenarios/holds.rs`,
  `crates/wakeshot-cli/src/args.rs`, `crates/wakeshot-cli/src/command.rs`, `crates/wakeshot-cli/src/main.rs`,
  `docs/architecture.md`, `docs/research.md` (new section at the end), `README.md`, `SECURITY.md`,
  `.github/ISSUE_TEMPLATE/bug-report.yml`, this card.
- Created: `crates/wakeshot-windows/src/diagnostics.rs`, `crates/wakeshot-cli/src/diagnostics.rs`.

**Checks**

- Tests that fail before their change: `rolls_while_running` (run on the old code: no rolled file);
  `each_session_gets_its_own_start_and_end_line` and `busy_session_holds_and_ends` (fail with the new lines turned
  off). The folder cap (`prune_keeps_the_folder_under_its_cap_and_drops_old_files`), the header
  (`the_start_header_names_this_pc`) and the bundle's privacy (`the_bundle_never_holds_a_planted_prompt_or_token`) test
  new code that did not exist before. The four-writer test now also rolls every 64 KiB: 32 files, every line whole,
  once, in order.
- Whole workspace, x64 and ARM64 (`CLAUDE.md`, Checks): all pass; `cargo test --workspace --locked` 1809 passed,
  0 failed, 64 ignored over 22 binaries (one more ignored test, `real_bundle_of_this_pc`, run by hand); no warning or
  error line; no CRT import. Exe sizes: x64 tray 3,179,520 bytes (+23,552), CLI 873,472 (+57,856).

**Install update** (as `p4-idle-cpu`): Haik's tray quit by `WM_CLOSE` (exit 0; his own lid Sleep AC and DC and sleep
18000 s AC and DC back, no request); `Setup.exe --silent` at Medium through Explorer, exit 0; installed exes equal the
build, `config.json` same bytes; tray 20124 and guard 50240 at Medium, parent explorer.exe; the new header and the
per-session line for the orchestrator's session are in the log; lid and sleep values after the start equal those
before the update (the tray's held values). Setup.exe (x64) 12,532,026 bytes, kept in
`<scratchpad>\p6-logs-diagnostics\pack-x64\`, SHA-256
`9048DA11EB9BC6754EA29ECEFE52C5DAC50DEA3E3D889DCF74C59048DA444984`.

**Open items**

- A File Explorer window or tab from the live `wakeshot diagnostics` check is open on Haik's screen, on a scratch
  folder now deleted. Haik can close it.
- The silent decisions the audit found in the watch and power code (Found outside scope 1 to 8), for a later card and
  the power safety reviewer.
- `docs/product.md` line 248 and the CLI list (Found outside scope 10), Haik's call.
