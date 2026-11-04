# Log wording: journal key names, a false WARN at quit, one log file name in the CLI

Phase: 6. Status: in progress. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 40K tokens. Written by the
orchestrator (session `wakeshot-12`, 2026-11-04) from `p6-int-dry-run-e2e` "Found outside scope" 1 and 2 and
`p6-int-wiring-fixes` "Found outside scope" 1 and 2.
Runs after: `p6-int-wiring-fixes` (done). Parallel with: cards that own none of the files below.
Real laptop: no.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p6-int-log-wording\target`.

## Goal

Three small log fixes, each with a test where one fits.

## Read first

- `docs/tasks/p6-int-dry-run-e2e.md` "Found outside scope" 1 and 2; `docs/tasks/p6-int-wiring-fixes.md` "Found outside
  scope" 1 and 2.
- `docs/architecture.md` section 8 (log lines: fields, plain English, no file content).

## Deliverables

1. `crates/wakeshot-windows/src/power/journal.rs`: the apply line prints the written keys as a Rust debug list
   (`wrote [LidAc, LidDc, SleepDc]`), and the "journal saved" lines spell them `LidAc` while the plan lines use
   `lidAc`. Use plain words and one spelling, the one the journal file and the plan lines use (`lidAc, lidDc,
   sleepDc`). Update the unit tests that assert the text.
2. `crates/wakeshot-windows/src/watch/thread.rs`: a post that fails with 1400 (the loop window is gone, as after a
   quit) logs a WARN "the next tick posts everything again", though no tick comes. Log it at Info, as the IO thread
   logs the same case ("as after a quit"). Test with the fake poster if the file has one.
3. `crates/wakeshot-cli/src/open_log.rs`: use `wakeshot_windows::log::log_file` instead of its own "wakeshot.log".
4. Optional, only if the cause is clear: `watch::thread::tests::a_process_exit_posts_the_session_and_the_hold_as_gone_at_once`
   failed once under load (`p6-int-wiring-fixes` Found outside scope 2: `rig.settled().len()` not 2 at
   `watch/thread.rs` about line 1070). If its settle wait is too tight for a loaded runner, widen it the way other
   tests in the file wait; otherwise leave it and say what you saw.
5. Docs lines that quote a changed log line (grep `docs/` for the old text): change only lines that describe what the
   app logs now; leave records of past runs (work logs, card Logs, dated research results) as they are.

## Acceptance checks

- `cargo fmt --all --check`; `cargo clippy --workspace --locked --target <t> -- -D warnings` with and without
  `--all-targets`, x64; ARM64 for `wakeshot-windows` and `wakeshot-cli` (the tray's ARM64 needs clang: CI only);
  `cargo test --locked -p wakeshot-windows` and `-p wakeshot-cli`; `cargo test --workspace --locked` once at the end.
- Build folder deleted at the end.

## Out of scope

- Any other log line; `app/*` files; core files.

## Questions for Haik

None.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-04: card written by the orchestrator (session `wakeshot-12`).
- 2026-11-04 04:51: worker started; read CLAUDE.md, plan "How we work", this card.

## Result

What was done, files changed, checks and outcomes, open items.
