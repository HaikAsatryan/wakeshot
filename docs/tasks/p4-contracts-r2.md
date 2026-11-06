# Contracts round 2: what the new agent sources share

Phase: 4. Status: in progress. Needs Haik: no.
Agent: `wakeshot-worker`. Size: about 200K tokens. If the card runs past about 220K, stop after a finished change
with its tests, and list the rest in the Result.
Runs after: `p4-codex-source`, `p4-copilot-source`, `p4-opencode-source` Part A, `p4-copilot-vscode-research` (all
done). Parallel with: the docs-only cards (`p4-antigravity-research`, `p4-hooks-design`) and `p4-live-harness` Part A.
It is the only card that builds inside the Rust workspace while it runs: it may change `Cargo.toml`, `Cargo.lock` and
the contracts.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-contracts-r2\target`; delete it when done.
Real laptop: no agent installs, no power settings, no screen.

## Goal

Every shared change the new v1 sources need, in one card run alone, so the later source cards (OpenCode Part B, VS
Code, Cursor, Antigravity) and live cards touch only their own files and build with `--locked`.

## Read first

- `CLAUDE.md` (Checks, cargo path, ARM64 note, elevation note), `docs/architecture.md` sections 2, 6, 7 and 11.
- `crates/wakeshot-core/src/sources.rs` (the contracts), `crates/wakeshot-windows/src/watch/` (`thread.rs`,
  `agents.rs`, `read.rs`, `dir.rs`, `data.rs`), `crates/wakeshot-windows/src/lockowner.rs`.
- `docs/tasks/p4-opencode-source.md`, Result ("Design for Part B": contract changes, the `sqlite.rs` module, the
  watch thread lines) and "Haik's answers".
- `docs/tasks/p4-codex-source.md`, "Found outside scope" 1 (reverted threads) and the Result.
- `docs/research/copilot-vscode-signals.md`, "Recommended detection rules" (rules 3, 6, 7) and "For the source card".
- `docs/research/cursor-gemini-signals.md`, the Cursor sections, only to name the stubs below.

## Deliverables

1. **`SourceId` variants** `OpenCode`, `VsCode`, `Cursor`, `Antigravity`, and a stub module for each under
   `crates/wakeshot-core/src/sources/` (`opencode.rs`, `vscode.rs`, `cursor.rs`, `antigravity.rs`): a module doc with
   the `Owner:` line (`p4-opencode-source`, `p4-vscode-source`, `p4-cursor-source`, `p4-antigravity-source`) and
   nothing that would warn. Fix every `match` on `SourceId` in the workspace. `Antigravity` stays even if its research
   says no live test is possible; Phase 8 removes it then.
2. **`WantedFile::Query`** as the OpenCode design names it (`db`, `name`, `sql`; JSON `{"rows":[...]}` or
   `{"error":"<kind>"}` through `Feed::Whole`; `Gone` when `<db>` or `<db>-wal` is missing; `SQLITE_BUSY` gives no
   feed). The watch thread plumbing: runs a `Query` want only while `<db>-wal` exists, on a change in the db's folder
   (at most one read per second) and on the safety tick. The thread calls one function in a new module
   `crates/wakeshot-windows/src/sqlite.rs` (`Owner: p4-opencode-source`), which this card leaves as a stub that
   returns the `no_sqlite` error; Part B of the OpenCode card fills it with the `winsqlite3.dll` loader. Tests with a
   fake query function: when a read runs, when it does not, the rate limit, `Gone`.
3. **`Liveness::ImageSince`** (`images`, `since_wall_ms`, the 2 s slack, ASCII case-insensitive image names, `None`
   means any start), checked in the watch thread from the system process list with no admin rights, with a
   `QUERY_LIMITED | SYNCHRONIZE` wait on each match so an exit re-checks at once. Tests with the fake liveness and one
   real-process test. Note in the doc that VS Code rule 7 (`Code.exe` started before the request time) fits this
   variant, so the VS Code source may use it without another contract change.
4. **`FindTail` for Codex reverted threads**: also match `-<thread id>_<rollout id>.jsonl` next to the plain
   suffix, and when several files match, follow the newest by modified time (feed `Gone` for the old one, then the
   new file from its start). Update `sources/codex.rs`, `tests/codex_source.rs` and the module doc lines that say
   reverted threads are not followed (no running card owns them now). Tests for both names and the switch.
5. **Whole-file reads of recently changed files** (VS Code rule 3): a read mode that gives `Feed::Whole` for files
   under a folder pattern (`workspaceStorage\*\chatSessions\*.jsonl`, `globalStorage\emptyWindowChatSessions\*.jsonl`)
   only when the file changed since the thread started or within the last 24 hours, read whole again on each change,
   `RetryOnce` on a partial last line, `Gone` on delete. A name filter runs before any open, like `Folder::accept`.
   Tests: old files never opened at start, a changed file read whole, compaction (rewrite in place) read whole.
6. Any `windows` crate feature the new sources need (process list for `ImageSince`, anything `sqlite.rs` will need
   for `LoadLibraryExW` and `GetProcAddress`, if not on yet), so no later card edits a `Cargo` file. List each one.
7. A stub module for installing agent hook folders, `crates/wakeshot-windows/src/agent_hooks.rs`
   (`Owner: p4-cursor-source`, module doc only), so the hooks build adds no module list line. If `p4-hooks-design`
   (running beside you) has already named another place by the time you reach this step, use its name.
8. `docs/architecture.md` sections 6 and 7: the new variants and read modes, in the same short style (re-read the
   file right before each edit; another card may add its own section at the same time; additions and edits to these
   two sections only).

## Acceptance checks

- Whole-workspace checks from `CLAUDE.md` on `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: `cargo fmt
  --all --check`; clippy with and without `--all-targets`, `-D warnings`; `cargo test --workspace --locked`; release
  build. The ARM64 run needs the Build Tools `VC\Tools\Llvm\x64\bin` folder on that command's PATH only. Record each
  result. After a `Cargo.lock` change, the first build may run without `--locked`; every check after it uses it.
- Every new behavior has a test that fails when the behavior is broken (say which ones you broke on purpose).
- No source reads anything it did not read before (Claude, Codex, Copilot tests pass unchanged except item 4).

## Out of scope

- The OpenCode, VS Code, Cursor and Antigravity parsers and `sqlite.rs`'s real body (their own cards).
- Registering sources in `crates/wakeshot-windows/src/app/tray_mode.rs` (the live cards do it).
- Hooks install code (design card first, then `p4-cursor-source`).
- `crates/wakeshot-windows/examples/agent-watch.rs` (owned by `p4-live-harness`).

## Rules

- Never run git, not even read-only (`git status`, `git diff`, `git --version`). Two workers slipped today.
- Never read Claude's `.key` files, OpenCode's `service.json` or any token file. Prompt text is never logged.
- Kill only processes you started, by PID. Start any test program at Medium through `explorer.exe` when the test
  depends on rights (`CLAUDE.md`, elevation note).
- Never set `RUSTFLAGS`. Cargo may not be on PATH: `%USERPROFILE%\.cargo\bin\cargo.exe`.
- Facts only from code or docs you read; mark guesses. Plain short English, no em dashes, no emojis. Write Log lines
  as you go.

## Questions for Haik

## Found outside scope

## Log

- 2026-11-06: card written by the orchestrator.
- 2026-11-06: worker started; reading the contracts, the watch thread and the design notes.

## Result
