# Integration: end-to-end tests of the real exes in dry run

Phase: 6. Status: blocked (waiting for a screen slot with the session unlocked: suite, Claude session and clean-up
done; pictures left). Needs Haik: no. Agent: `wakeshot-worker`. Size: about 130K tokens.
Real laptop: Part A no (code). Part B: **screen** (a dry-run tray icon, its balloons, pictures; one screen or sound card
at a time, across all tracks) and the **agent-session lane** (one Claude Code test session, as `p4-live-claude` ran
them). No power setting is written: the dry-run tray uses `DryRunPower`, dry-run requests, lock and hibernate.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-int-dry-run-e2e\target`.
Runs after: `p6-int-update-wiring`, `p4-cli-args`, `p4-cli-run`, `p4-cli-commands`, `p4-cli-restore`,
`p4-live-claude`. Parallel with: code cards that own none of the files below; no other screen or sound card in Part B.

## Goal

Prove the two exes work together as the user runs them, without touching power settings: every CLI command against a
running dry-run tray, Keep awake, `wakeshot run`, Automatic, restore, quit, a crash and the next start, a Claude Code
session, and the real exe's look (balloon header, exe icon).

## Read first

- `docs/phases/06-integration.md`: notes 3, 6 and 7.
- `docs/product.md`, "UX" (the CLI commands and what each says); `docs/hold-files.md`; `docs/architecture.md`
  section 4 ("How they talk", "CLI").
- `docs/phases/05-tray-ux.md` note 4 (Medium start, dry-run GUID, pictures, the `NotifyIconSettings` clean-up) and
  note 5 (real-exe checks).
- The Results of `p6-int-tray-mode` (the smoke test), the CLI cards, `p4-live-claude` (how a Claude Code test session
  is started and ended).

## Deliverables

1. `crates/wakeshot-tray/tests/e2e/mod.rs`, owned whole (passes to `p6-int-real-e2e` after this card): start the tray
   at Medium through `explorer.exe <launch .cmd>` with `WAKESHOT_DATA_DIR`, find its pid by exe path, run
   `wakeshot.exe` (next to the tray exe; build `wakeshot-cli` first) with arguments and the same env, wait for a file
   or a `state.json` field with a timeout, read the log, quit with `ask_quit`, end by pid.
2. `crates/wakeshot-tray/tests/e2e_dry_run.rs`, owned whole, every test `#[ignore]` and named `real_e2e_*`:
   - start: `state.json` within 10 s; a second start exits 0 at once and the first keeps running;
   - first run: with an empty data folder the `FirstRun` notice is logged once; a second start logs none;
   - `keep-awake 30m`, then `off`; `keep-awake on --screen`: `state.json` holding with the Keep awake label, the
     dry-run log shows the requests (display only with `--screen`) and the dry-run apply; off gives idle and a restore;
   - `run -- cmd /d /c "ping -n 4 127.0.0.1 >nul"`: a `run-<pid>.json` hold while it runs, holding in `state.json`,
     then idle; the exit code passes through (`cmd /d /c exit 3` gives 3);
   - `run` whose wrapper is ended by pid: the hold drops within one safety tick (Gone);
   - `automatic off`: a run hold no longer counts, Keep awake still does; `automatic on`;
   - `status` while running and after quit ("not running");
   - `restore` with the tray running: `restore-request.json` is gone within 5 s and the CLI prints the result;
   - quit with `ask_quit` during Keep awake: the Keep awake hold file and the dry-run journal are gone; exit code 0;
   - the tray ended by pid during a hold, then started again: the leftover is handled at start (the log says so);
   - a log line check: every line has the section 8 fields and reads as plain English; no file content.
3. Part B, in a screen slot (and the agent-session lane):
   - run the suite: `cargo build --locked -p wakeshot-cli --target x86_64-pc-windows-msvc`, then
     `cargo test --locked -p wakeshot-tray --target x86_64-pc-windows-msvc --test e2e_dry_run -- --ignored
     --test-threads=1`;
   - one Claude Code test session (Haiku, a throwaway folder, the `p4-live-claude` method) with one tool call that
     runs about 60 s: the dry-run tray holds for it and lets go after it. Other Claude sessions on this laptop (the
     orchestrator's) may hold too: record them, never stop them;
   - pictures (`docs/research/img/ux/int-*.png`, cropped): the taskbar icon idle and holding; the first-run balloon
     with its header (the header must say "Wakeshot", from `FileDescription`); the exe in Explorer with its icon;
   - clean up: export and delete the dry-run GUID's `NotifyIconSettings` entry and its `NotifyIconGeneratedAumid_`
     key, check both gone a minute later; no process this card started is left (by pid); temp data folders deleted.
4. Fixes the run needs, only in the `app/*` files, `velo.rs`, `tests/smoke.rs` and the CLI command files (phase file
   note 1); anything else under "Found outside scope".
5. `docs/research.md`: a section "Integration dry-run e2e" with what was checked and how.

## Acceptance checks

- Tests first: write the suite before any fix; after the fixes every `real_e2e_` test passes (record the run's output
  in the Log).
- The Claude session check and the pictures are recorded; the clean-up checks pass.
- `rustfmt --edition 2024 --check` on the owned files and any fixed file.
- `cargo clippy --locked -p wakeshot-tray --all-targets --target <t> -- -D warnings` and without `--all-targets`; the
  same for each other package with a fixed file; both targets (the tray's ARM64 needs clang).
- Plain `cargo test --locked -p wakeshot-tray` and `-p wakeshot-windows -- app::` still pass (no ignored test runs).
- Build folder deleted at the end.

## Out of scope

- Real power settings (`p6-int-real-e2e`); installs (`p6-int-update-e2e`); `wakeshot log` (it opens a window in the
  default app; `p4-cli-commands` checked it); Codex and Copilot sessions (their live cards); any track file other than
  those in deliverable 4.

## Questions for Haik

None.

## Found outside scope

Problems seen but not fixed.

1. `power/journal.rs`, log text only: the apply line prints the written keys as a Rust debug list, `apply to plan
   <guid> done; wrote [LidAc, LidDc, SleepDc]; 3 journal entries`. The "journal saved" lines name keys `LidAc` while
   the plan lines name them `lidAc`. Plain words and one spelling (`lidAc, lidDc, sleepDc`, as the file and the plan
   lines use) would read better. Minor.
2. `watch/thread.rs`, log level and text: when a hold file changes at a quit, the watch thread's post fails with
   1400 after the loop window is gone. It then logs a WARN: "an event could not be posted to the loop (PostMessageW
   error 1400); the next tick posts everything again". Nothing is wrong, and no tick comes after a quit. It happened
   at 1 of 11 quits in run 1 and 3 of 12 in run 2, each time when a hold file changed at the quit: Keep awake's file,
   which the quit itself deletes, or a short `run`. So a user who quits during Keep awake can get this WARN. The IO
   thread logs the same case as Info, "as after a quit". Fix: log 1400 as the IO thread does (watch side). Or the
   tray could stop the watch thread at Quit, before the window is destroyed, as it already does for the session
   registration (`app/tray_mode.rs`; that changes the order in phase note 6). Not fixed here: the run does not need
   it, and the clean fix is in the watch thread's file. Minor.

## Log

- 2026-11-03: card written by `p6-plan-integration`.
- 2026-11-04 04:01: started by `wakeshot-worker`. Two stops: stop 1 writes the suite (deliverables 1 and 2) and
  makes it compile; stop 2 runs it when the laptop lane is free.
- 2026-11-04 04:19: stop 1 done, before any fix and without any run. `tests/e2e/mod.rs` (harness) and
  `tests/e2e_dry_run.rs` (12 tests, all `#[ignore]`, all `real_e2e_*`) written. `rustfmt --edition 2024 --check` on
  both, clippy x64 `-p wakeshot-tray` with and without `--all-targets` clean; `wakeshot-cli` built (x64) into the
  card's target folder; plain `cargo test --locked -p wakeshot-tray`: 8 + 5 pass, the 12 new ones ignored. No tray
  started. Decided by the worker, for Haik's review (none changes the product):
  1. The CLI runs as the test's own child (High in an agent shell; a user's terminal is Medium), so its output and
     exit code come back directly; the tray stays Medium, so files cross the two levels. Change it when a check needs
     a Medium CLI: `spawn_outside_tree` under `cmd /c` with output to files.
  2. "`run` whose wrapper is ended by pid": `wakeshot run` puts its child in no job, so ending the wrapper leaves the
     child running and its hold rightly stays. The test checks that, then ends the child by pid and checks the hold
     drops within one safety tick (Gone) and the tray deletes the file.
  3. The second start runs through Explorer too (Medium), in a `.cmd` that waits for it and writes its exit code;
     "at once" is checked as under 10 s including Explorer and `cmd`, plus no second start line in the log.
  4. "Reads as plain English" is checked by rule (no Rust debug form, escaped path, JSON file content, control
     character; at least one word) and by reading the printed log at the run; no ERROR line in a plain session.
  5. The Claude Code session check is a 12th test, `real_e2e_claude_session_timeline`, which does nothing unless
     `WAKESHOT_E2E_CLAUDE_WATCH_S` is set: it starts the tray on the real `~\.claude` and prints every change of the
     holding list with its time, to read against the test session (other sessions count too).
- 2026-11-04 04:37: stop 2 started (suite committed in 666f05b; `p6-int-wiring-fixes` committed in 7b6d076). Before
  the run: no `wakeshot-tray.exe` or `wakeshot.exe` runs; 16 `NotifyIconSettings` entries, none with a Wakeshot
  path; 2 `NotifyIconGeneratedAumid_` keys; no `%LocalAppData%\Wakeshot`. Lists saved in the scratchpad.
- 2026-11-04 04:38: run 1 of the suite (CLI and tray rebuilt on 7b6d076, x64 debug, `--ignored --test-threads=1
  --nocapture`, 04:37:56 to 04:38:44): **12 passed, 0 failed** in 44.2 s, before any fix (the Claude timeline test
  skipped itself: no variable). So no fix was needed. Timings: first state 0.52 to 0.60 s after the launch, but 5.5 s
  for the first test of the run; second start exit 0 after 2.5 s (Explorer and `cmd` included); run hold file 58 ms
  after `wakeshot run` started; state follows in 0.9 to 1.0 s; a killed `ping` dropped its hold in 62 ms (exit wait,
  not the tick); `wakeshot restore` answered in 136 ms; "notice shown: FirstRun" once, none at the second start. The
  log line test: 42 lines (tray 34, cli 8), all Info, no problem by rule. Read by eye: two readability points in the
  journal lines and one WARN at another test's quit (Found outside scope 1 and 2). Output:
  `<scratchpad>\p6-int-dry-run-e2e\run1.txt`.
- 2026-11-04 04:41 to 04:43: Claude Code test session (2.1.318, `claude -p --model haiku --allowedTools Bash
  --output-format json` in a throwaway `%TEMP%` folder, CLAUDE* variables dropped as `ccdrive.py` does), with
  `real_e2e_claude_session_timeline` watching the real `~\.claude` for 180 s. Three other sessions (the
  orchestrator's and others) held the dry-run tray all along: "Claude Code (3 sessions)"; recorded, not touched.
  Try 1 (pid 37148): Claude Code refused a foreground `sleep 60` and told the model to use a background task, so
  the session ended after 12 s (4 sessions 00:41:34Z to 00:41:44Z). Try 2 (pid 33060), the tool call `ping -n 61
  127.0.0.1 > /dev/null`: started 00:42:00.6Z, status file 00:42:01.3Z, result "done" after 66.6 s, exit 0 at
  00:43:08.3Z; the tray went to 4 sessions at 00:42:01Z and back to 3 at 00:43:07Z. So the dry-run tray holds for
  the session from its first status write and lets go when it goes idle, 1 s before the process exits. Both
  sessions' status files were gone at their exit. Their project folders, `session-env` folders and work folders
  (six, all made by these two sessions) deleted and checked gone; both pids gone.
- 2026-11-04 04:44 to 04:45: run 2 of the suite: **12 passed** in 36.2 s (first launch 1.1 s; Gone 63 ms; restore
  136 ms; second start 2.5 s). The watch thread's WARN at a quit came 3 times in 12 quits (Found outside scope 2).
  Decided by the worker, for Haik's review: not fixed here. The run does not need it, and the clean fix is in
  `watch/thread.rs`, outside this card's files. Output: `<scratchpad>\p6-int-dry-run-e2e\run2.txt`.
- 2026-11-04 04:46 to 04:48: clean-up. The dry-run icon: one new `NotifyIconSettings` entry by id comparison,
  `15983757285637097514`, with only `IconSnapshot`. It came with `NotifyIconGeneratedAumid_15983757285637097514`
  (`DisplayName` "Wakeshot"). Both were exported to the scratchpad (`notifyicon-*.reg`, `aumid-*.reg`), deleted at
  04:46:23, and still gone at 04:48:38 with no new entry. The shell's
  `%TEMP%\NotifyIconGeneratedAumid_15983757285637097514.png` is left: Windows wrote it, not this card. Also checked:
  no `wakeshot-tray.exe`, `wakeshot.exe` or `PING.EXE` runs, no `%TEMP%\wakeshot-e2e-*` folder is left, there is no
  `%LocalAppData%\Wakeshot`, and `Run` and `RunOnce` hold no Wakeshot value.
- 2026-11-04 04:47 to 04:49: checks on 7b6d076. `rustfmt --edition 2024 --check` on both files passes. Clippy x64 on
  `-p wakeshot-tray` is clean with and without `--all-targets`. ARM64 clippy fails at `ring`, which needs clang, and
  this laptop has none, so it runs in CI. Plain `cargo test --locked -p wakeshot-tray` passes 8 + 5, with 15 ignored.
  `cargo test --locked -p wakeshot-windows -- app::` passes 103. `docs/research.md` has the new section
  "Integration dry-run e2e".
- 2026-11-04 04:50: build folder deleted and checked gone. Status `blocked`: the pictures wait for a screen slot with
  the session unlocked (steps below).

## Steps left: the pictures (deliverable 3)

For Haik, or an agent in a screen slot once the session is unlocked. No other dry-run tray may run, and no power
setting is written.

1. Unlock the session. Build into `<scratchpad>\p6-int-dry-run-e2e\target`: `cargo build --locked -p wakeshot-tray
   -p wakeshot-cli --target x86_64-pc-windows-msvc`.
2. List the ids under `HKCU\Control Panel\NotifyIconSettings` and the `NotifyIconGeneratedAumid_*` keys under
   `HKCU\Software\Classes\AppUserModelId`.
3. Make an empty folder `%TEMP%\wakeshot-pictures`. Write a `.cmd` that sets `WAKESHOT_DATA_DIR` to it and runs
   `start "" "<target>\x86_64-pc-windows-msvc\debug\wakeshot-tray.exe" --dry-run`. Run it with `explorer.exe <the
   .cmd>`: the tray starts at Medium, as a first run.
4. The first-run balloon shows: capture it with its header, which must say "Wakeshot". Save it as
   `docs/research/img/ux/int-first-run-balloon.png`, cropped.
5. Capture the taskbar icon while idle: `int-icon-idle.png`.
6. In a terminal with the same `WAKESHOT_DATA_DIR`, run `wakeshot --dry-run keep-awake on`. Capture the icon while
   holding: `int-icon-holding.png`. Then run `wakeshot --dry-run keep-awake off`.
7. Quit from the tray menu.
8. Open the build folder in Explorer and capture `wakeshot-tray.exe` and `wakeshot.exe` with their icons:
   `int-exe-icon.png`.
9. Clean up. Compare the ids with step 2, export the new `NotifyIconSettings` entry and its
   `NotifyIconGeneratedAumid_` key, then delete both, and check they are still gone a minute later. Delete
   `%TEMP%\wakeshot-pictures` and the build folder.

## Result

**Done except the pictures (2026-11-04 04:50); Status `blocked` for them.** No fix was needed: the suite passed on
its first run.

What was done:

- Deliverables 1 and 2: `crates/wakeshot-tray/tests/e2e/mod.rs` and `crates/wakeshot-tray/tests/e2e_dry_run.rs`.
  The harness starts the tray at Medium through Explorer on a temp data folder and finds it by its exe path. It runs
  `wakeshot.exe` beside it, waits on `state.json` and the log, quits with `ask_quit` (after checking the loop
  window's pid) and ends processes by pid. The suite has 12 ignored `real_e2e_*` tests; each item of the card is one
  test, plus the env-gated Claude timeline.
- Deliverable 3: two full runs, **12 of 12 passed both times** (44.2 s and 36.2 s; output in
  `<scratchpad>\p6-int-dry-run-e2e\run1.txt` and `run2.txt`). The Claude Code session is recorded: the tray went from
  3 to 4 sessions and back with the test session (Log 04:41). The clean-up checks pass. **Left: the pictures**, since
  the session was locked (steps above).
- Deliverable 4: no fix, so no app, `velo.rs`, `smoke.rs` or CLI file changed. Two minor log findings are under
  "Found outside scope".
- Deliverable 5: `docs/research.md`, section "Integration dry-run e2e".

Files changed: the two test files, this card, and `docs/research.md` (additions only).

Checks: rustfmt passes. Clippy x64 is clean with and without `--all-targets`. ARM64 clippy for the tray needs clang
and runs in CI only. The plain tray tests pass (8 + 5), and so do the `wakeshot-windows` `app::` tests (103). The
build folder is deleted.

Decided by the worker, for Haik's review: Log 04:19 items 1 to 5, and Log 04:44 (the WARN left).

Open items: the pictures (steps above) and Found outside scope 1 and 2. `p6-int-real-e2e` takes over
`tests/e2e/mod.rs`; it is dry run only now, so that card adds the real mode.
