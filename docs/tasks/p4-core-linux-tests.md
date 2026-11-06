# Core tests pass on Linux again (CI `core-linux`)

Phase: 4. Status: done. Needs Haik: no.
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

- CI's `core-linux` runs `cargo test` without `--no-fail-fast`, so one failing test binary hides every later one
  (this is how the VS Code failures below stayed hidden). Added `--no-fail-fast` at the orchestrator's request
  (Log).
- `tests/vscode_source.rs` (owner `p4-live-vscode`) fails on Linux by reading; see Result. Not edited.

## Log

- 2026-11-06: card written by the orchestrator from CI run on 98c8795.
- 2026-11-06: worker started; reading the Codex and OpenCode tests and sources.
- 2026-11-06: WSL `Ubuntu-26.04` has no C compiler or libc dev files (no `cc`, no `crt1.o`), so Rust there would
  also need a linker without sudo (plan: Zig 0.17.1 tarball as `cc`). The install script (rustup-init from
  static.rust-lang.org with its sha256 check, plus the Zig tarball) was refused by the agent's permission check, so
  nothing was installed in WSL (nothing to remove). Per the card: fix by reading the tests; CI checks.
- 2026-11-06: cause in `codex_source.rs`: rollout paths joined `r"2026\11\06"` as one piece, so on Linux a rollout
  sat 2 levels under `sessions`, `rollout_id` (4 parents up) never matched, and every rollout feed was ignored; the
  15 failing tests are exactly the ones that expect a rollout to change a session. Fixed with `day_dir`/`date_dir`
  (one `join` per level) and `profile.join(".codex")`. `opencode_source.rs`: only
  `the_database_is_in_the_xdg_data_folder` had joined strings; now one `join` per level. Windows: both binaries
  pass (22 and 15), rustfmt clean. Next: read the other core tests for Linux failures.
- 2026-11-06: CI log of job 127874604221 (`gh api`, run from the scratch folder, no git) lists the same 15 failing
  Codex tests I had predicted from reading. Read `copilot_source.rs`, `vscode_source.rs`, the core sources'
  path code and every `r"...\..."` string in core: only VS Code has more Linux failures (3 tests). The hooked and
  Cursor sources are still 7- and 5-line stubs with no tests. No doc tests in core.
- 2026-11-06: Windows checks pass (all core tests, clippy all targets). Facts added to `docs/research.md` ("Core
  tests on Linux: Windows path strings"). Build folder and scratch files deleted; WSL home has no `.cargo`,
  `.rustup` or work folder.
- 2026-11-06: orchestrator follow-up: `.github/workflows/ci.yml`, `core-linux` step "Test the core", now runs
  `cargo test -p wakeshot-core --locked --no-fail-fast`, so one failing test file no longer hides the others.
  Nothing else in the file changed.

## Result

Status: done (2026-11-06). Linux was not run here: the WSL install was refused (Log), so the fix is by reading,
checked against the CI log; CI's next `core-linux` run confirms it.

What was done:

- `crates/wakeshot-core/tests/codex_source.rs`: new helpers `day_dir(day)` and `date_dir(root, day)` build
  `sessions\2026\11\<day>` with one `join` per level; every `join(r"2026\11\06")`, `r"2026\11\07"` and
  `r"archived_sessions\2026\11\06"` now uses them. The home-folder test expects `profile.join(".codex")`. On Windows
  every path is the same string as before, so no check got weaker.
- `crates/wakeshot-core/tests/opencode_source.rs`: `the_database_is_in_the_xdg_data_folder` builds its expected
  paths with one `join` per level.
- No fixture changed. Made-up roots (`X:\home\.codex`, the OpenCode `DB`, `C:\Users\x`) stay as one opaque piece,
  the way the Claude tests do it. Change it when a source starts to look at the parts of its root path.
- Sources: `codex_home`, `CodexSource` and `opencode_db` build paths with `join` only and compare whole paths, so
  nothing differs by OS in a way that matters on Windows.

Checks (Windows, `CARGO_TARGET_DIR` in the scratch folder): `cargo test -p wakeshot-core --locked` all pass
(lib 709 + 1 ignored, claude_status 11, claude_transcript 25, codex_source 22, copilot_source 42, opencode_source 15,
vscode_source 31); `cargo clippy -p wakeshot-core --all-targets --locked -- -D warnings` clean; `rustfmt --edition
2024 --check` on my two files clean. Linux: not run (see above).

Other core tests that fail on Linux (by reading, not run):

- `tests/vscode_source.rs` (owner `p4-live-vscode`), 3 tests. Cause: `USER` and `INSIDERS_USER` are one string
  each (`r"X:\Users\me\AppData\Roaming\Code\User"`), but `installs()` joins `ROAMING`, `Code`, `User` level by
  level, so on Linux the two never compare equal.
  - `it_wants_both_chat_folders_of_each_build_whole_and_recent`: wanted dirs from `both_builds()` vs `USER`.
  - `installs_find_the_data_folder_the_way_vs_code_does`: expects `PathBuf::from(r"X:\Roaming\Code\User")` and the
    like, and the `USER` fallback.
  - `an_insiders_chat_lives_by_the_insiders_image`: feeds under `INSIDERS_USER` are not located by the roots
    `both_builds()` made, so `only()` finds no session.
  Fix for the owner: build `USER` and `INSIDERS_USER` as functions with `join` (`Path::new(ROAMING).join("Code")
  .join("User")`) and the expected paths in the `installs` test the same way.
- Pass on Linux: lib unit tests, `claude_status.rs`, `claude_transcript.rs` (ran in CI on 98c8795);
  `copilot_source.rs` and the rest of `vscode_source.rs` (by reading). Hooked and Cursor: no tests yet in core.

Open items: none for this card. The orchestrator relays the VS Code list to `p4-live-vscode`.
