# Live test: Copilot agent mode in VS Code

Phase: 4. Status: ready once `p4-vscode-source` and the live card before it in the lane are done (new 2026-11-06).
Needs Haik: no (agents run every step; VS Code Copilot is signed in on Free).
Agent: `wakeshot-worker`. Size: about 120K tokens.
Runs after: `p4-vscode-source`, `p4-live-harness`; one live card at a time (agent-session lane).
Parallel with: build cards that own none of the files below; at most four Rust cards at once.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-live-vscode\target`; delete it when done.
Real laptop: VS Code (per machine, Haik's profile, Copilot Free). No power settings, no screen captures.

## Goal

The live test of the VS Code chat source: the rules of `docs/research/copilot-vscode-signals.md` and the build of
`p4-vscode-source` on a real VS Code, its answers to "Open questions only a live test answers", redacted fixtures,
fixes with tests, and the source wired into the tray.

## Read first

- `docs/research/copilot-vscode-signals.md` (all, mainly the open questions), `docs/tasks/p4-vscode-source.md`
  Result, `docs/tasks/p4-live-harness.md` Result (runbook, driver, whether a chat starts from the command line).
- `docs/tasks/p4-live-claude.md` Result (the shape of a live result), `crates/wakeshot-windows/src/app/tray_mode.rs`.

## Deliverables

1. Before any step: is VS Code already running for Haik? If it is, never close, kill or type into his windows: open
   your own window only if it is a separate process tree you can end by PID, else skip the crash step and mark it.
2. Sessions at Medium, each timed (send to post, end to post): a working agent mode turn (a command that runs about
   60 s), an approval wait left 1 minute then approved, a stop (Esc or the Stop button through the command line or
   the driver), a clean end, a compaction if cheap (the 1024 line rewrite: mark "not reached" if it costs too many
   prompts), an empty window chat, and a crash (only per item 1). How fast the file shows `{"value":0}` after a send
   and `completedAt` after the end (the research says about once a minute).
3. Run `agent-watch` with a new `--source vscode` row (`crates/wakeshot-windows/examples/agent-watch.rs`, one row in
   `SOURCES`) and record what it posted per step.
4. Fixtures: redacted lines into `crates/wakeshot-core/tests/fixtures/vscode/live/` (no prompt text, no paths with
   the user name, fake ids), a test that replays them, and fixes in `sources/vscode.rs` each with a test that failed
   before.
5. Wire the source into `crates/wakeshot-windows/src/app/tray_mode.rs` and test it through the app with `--dry-run`
   (never beside another dry-run card; never quit Haik's installed tray).
6. A section "Phase 4 live test (2026-11-..)" in `docs/research/copilot-vscode-signals.md` and short lines in
   `docs/research.md`: what holds, the delays, what was verified, the VS Code and Copilot versions; anything not run
   stays "not tested live".

## Rules for every live card of the bigger v1 (2026-11-06)

- Minimum tokens (Haik): each prompt says it is an app test and asks for one word or `0`, no reasoning; the lowest
  reasoning setting and the smallest model the plan allows. Count prompts in the Log; free plans have monthly limits.
- Start agents and the driver at Medium through `explorer.exe`. Kill only processes you started, by PID.
- A folder listing can show a stale size and time for a file another process holds open (`p4-live-harness`): time
  each step from the write to the post.
- Checks: `cargo fmt` on your own files; `cargo clippy --locked -p wakeshot-core -p wakeshot-windows --all-targets
  -- -D warnings` for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc` (ARM64 needs the Build Tools
  `VC\Tools\Llvm\x64\bin` folder on that command's PATH only), the same without `--all-targets`; `cargo test --locked
  -p wakeshot-core -p wakeshot-windows`. A failure in another card's file is reported, never fixed.
- Never run git, not even read-only (`git status`, `git diff`, `git --version`). Never read token files or app
  secrets. No contract or `Cargo` change: stop and tell the orchestrator. Plain short English, no em dashes, no
  emojis. Write Log lines as you go.

## Owns

`crates/wakeshot-core/tests/fixtures/vscode/live/`; fixes in `sources/vscode.rs` and `tests/vscode_source.rs`; the
VS Code line in `tray_mode.rs` and in `agent-watch.rs`; the research sections above.

## Questions for Haik

## Found outside scope

## Log

- 2026-11-06: card written by the orchestrator.

## Result
