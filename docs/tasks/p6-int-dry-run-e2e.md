# Integration: end-to-end tests of the real exes in dry run

Phase: 6. Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 130K tokens.
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

## Log

- 2026-11-03: card written by `p6-plan-integration`.

## Result

What was done, files changed, checks and outcomes, open items.
