# Core tests pass on Linux again (CI `core-linux`)

Phase: 4. Status: ready (new 2026-11-06 ~19:55). Needs Haik: no.
Agent: `wakeshot-worker`. Size: about 60K tokens.
Runs after: nothing. Parallel with: the running cards that own none of the files below; at most four Rust cards.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-core-linux-tests\target` (Windows); inside WSL its own target folder
under the WSL home. Delete both when done.
Real laptop: WSL `Ubuntu-26.04` may get a Rust toolchain for Haik's user (no sudo). No power settings, no screen.

## Goal

CI's `core-linux` job (`cargo test -p wakeshot-core --locked` on `ubuntu-latest`) has failed since fb0ebce: 15 of
22 tests in `crates/wakeshot-core/tests/codex_source.rs` hard-code Windows path strings (for example left
`"C:\\Users\\x/.codex"`, right `"C:\\Users\\x\\.codex"`: `Path::join` uses `/` on Linux). `cargo test` stops at the
first failing test binary, so the later binaries (Copilot, OpenCode, VS Code, hooked) never ran on Linux. Make every
core test pass on Linux and on Windows without weakening what it checks.

## Deliverables

1. A way to run the core tests on Linux here: in WSL `Ubuntu-26.04` (Rust is not installed there), install the pinned
   toolchain for Haik's user without sudo if the tools allow it (`rustup` into `~/.cargo`; log the remove command),
   and run `cargo test -p wakeshot-core --locked --no-fail-fast` on a copy of the repo in the WSL home (never build
   into the Windows tree from WSL). If an install is refused, say so and fix by reading the tests; CI then checks.
2. Fix `crates/wakeshot-core/tests/codex_source.rs` and `crates/wakeshot-core/tests/opencode_source.rs` (and their
   fixtures if needed): build expected paths with `Path::join` or compare path parts, the way the Claude tests do. If
   a source itself builds paths in a way that differs by OS in a way that matters on Windows, say so; do not change a
   source's Windows behavior.
3. List every other core test file that fails on Linux, with the failing tests, for the orchestrator: those files
   belong to running cards (`p4-live-copilot` owns `tests/copilot_source.rs`, `p4-live-vscode` owns
   `tests/vscode_source.rs`, `p4-cursor-source` owns the hooked and Cursor tests). Do not edit them.

## Acceptance checks

- On Linux (WSL), if available: `cargo test -p wakeshot-core --locked --no-fail-fast`: the files you own pass; the
  others listed. On Windows: `cargo test -p wakeshot-core --locked`; `cargo clippy -p wakeshot-core --all-targets
  --locked -- -D warnings`; `cargo fmt` on your files.

## Out of scope

- Any source's behavior on Windows. Test files other cards own. CI files.

## Rules

- Never run git, not even read-only (`git status`, `git diff`, `git --version`); copy the repo into WSL with a plain
  file copy, without `.git` and `target`.
- Never read token files or app secrets. Kill only processes you started, by PID. Never set `RUSTFLAGS`. Cargo on
  Windows: `%USERPROFILE%\.cargo\bin\cargo.exe`.
- Plain short English, no em dashes, no emojis. Write Log lines as you go.

## Questions for Haik

## Found outside scope

## Log

- 2026-11-06: card written by the orchestrator from CI run on 98c8795.

## Result
