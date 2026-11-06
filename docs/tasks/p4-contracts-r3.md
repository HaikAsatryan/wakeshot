# Contracts round 3: what the agent hooks build shares

Phase: 4. Status: done (2026-11-06). Needs Haik: no.
Agent: `wakeshot-worker`. Size: about 90K tokens.
Runs after: `p4-contracts-r2`, `p4-hooks-design`, the security review `docs/reviews/p4-hooks-design-security.md`
(its fixes to the design applied first). Parallel with: docs-only and agent-session cards. It is the only card that
builds inside the Rust workspace while it runs.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-contracts-r3\target`; delete it when done.
Real laptop: no installs, no power settings, no screen.

## Goal

The shared changes the Cursor and Antigravity hooks build needs, in one card run alone, so `p4-cursor-source`,
`p4-agent-hooks-install` and `p4-antigravity-source` touch only their own files and build with `--locked`.

## Read first

- `CLAUDE.md` (Checks, cargo path, ARM64 note), `docs/architecture.md` sections 2, 6 and 12.
- `docs/agent-hooks.md`, all of it; "Contract changes the build needs" is your list.
- `docs/reviews/p4-hooks-design-security.md`: every finding that changes a contract.
- `docs/tasks/p4-contracts-r2.md` Result (what round 2 added).

## Deliverables

Every item of "Contract changes the build needs" in `docs/agent-hooks.md`, as the design says after the review's
fixes (six items on 2026-11-06 ~16:40: `values` in `Command::Hook`, `current_above_medium()` and
`harden_dll_search()`, and `write_atomic` with `CREATE_NEW` for every caller were added). The list below is the
first version; where it differs, the design wins:

1. `wakeshot-cli`: `Command::Hook { agent, event }` in `command.rs`, its line in `main.rs`, the parse in `args.rs`
   (hidden from help), a stub `hook.rs` (`Owner: p4-cursor-source`) that prints `{}` and exits 0, so the command is
   safe even before the build.
2. `wakeshot-windows::process`: `ProcessEntry.parent: u32` and `image_path(pid)`.
3. `wakeshot-core`: `Event::HooksAdded`, `Notice::HooksAdded`, the core's answer, the words in `text.rs` (the design
   gives them; house tone).
4. `wakeshot-windows::hooks`: `HookEnv::remove_agent_hooks`, called in `on_uninstall` after `remove_notices`, with a
   stub body and the fake updated.
5. A stub `wakeshot-core/src/sources/hooked.rs` (`Owner: p4-cursor-source`); the `Owner:` line of
   `crates/wakeshot-windows/src/agent_hooks.rs` set to `p4-agent-hooks-install`.

6. **OpenCode review major 1** (`docs/reviews/p4-opencode-db-read.md`; added by the orchestrator 2026-11-06 ~18:10,
   since it is watch thread plumbing from `p4-contracts-r2` and no other card may edit `watch/*` now): a `Query` want
   re-runs only on change notices for `<db>-wal` or `<db>`, never `-shm`, so a crash-left `-shm` cannot loop reads.
   A test that fails before the fix (the reviewer's probe scripts are in
   `<scratchpad>\p4-opencode-review\scripts\`, data only).

7. `docs/architecture.md` section 7: the Codex "can outlive its session" mark now applies only to the fallback when
   Restart Manager fails (`docs/tasks/p4-live-codex.md` Result, Restart Manager named the session's own `codex.exe`).

8. (Added by the orchestrator 2026-11-06 ~18:50.) `crates/wakeshot-windows/src/app/tray_mode.rs`: start the Copilot
   CLI, VS Code chat and OpenCode sources the way `p4-live-codex` added Codex, each with a unit test like Codex's and
   its own home rules. Not Cursor or Antigravity.
9. (Added the same time.) `crates/wakeshot-windows/examples/agent-watch.rs`: a `SOURCES` row each for `vscode` and
   `opencode`.

Tests for each new piece that has behavior (the parse, the stub command's output and exit code, the parent field,
the core's answer to `HooksAdded`). `docs/architecture.md`: lines in sections 6 and 12 for what you added (re-read
right before each edit).

## Acceptance checks

- Whole-workspace checks from `CLAUDE.md` on `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`, every one with
  `--locked`. The ARM64 run needs the Build Tools `VC\Tools\Llvm\x64\bin` folder on that command's PATH only. Record
  each result.
- No `Cargo` change, unless the review's fixes need one: then say why in the Log.

## Out of scope

- The hook command's real body, the state files, the sources, the plugin files and the install code (the build
  cards).

## Rules

- Never run git, not even read-only (`git status`, `git diff`, `git --version`).
- Never read token files or any app's secrets. Prompt text is never logged.
- Kill only processes you started, by PID. Never set `RUSTFLAGS`. Cargo may not be on PATH:
  `%USERPROFILE%\.cargo\bin\cargo.exe`.
- Facts only from code or docs you read; mark guesses. Plain short English, no em dashes, no emojis. Write Log lines
  as you go.

## Questions for Haik

## Found outside scope

1. The design said no `windows` feature was needed; `NtQueryInformationProcess` needs `Wdk_System_Threading`. Added
   here (contract item 2) and the design line fixed.
2. A crashed OpenCode's leftover files are still read on each 30 s safety tick (three queries, each rebuilding the
   `-shm`): bounded, not a loop. The review's stronger option (no `Query` run while no `opencode.exe` lives, a
   contract field) is open; change it when the live card shows leftovers are common.
3. The tray now watches `%APPDATA%\Code\User\workspaceStorage` with its subtree for VS Code chat, which VS Code
   writes often for other files; each such notice only runs the name filter. Its cost is not measured: for
   `p4-live-vscode`.
4. What Copilot CLI does with an empty `COPILOT_HOME` is not verified; the tray treats it as unset, as the harness
   does.
5. `write_atomic` still deletes its own temp file by name after a failed write or move. At Medium that gains an
   attacker nothing; for an elevated `wakeshot run` it is the same junction gap the design accepts (contract 6).

## Log

- 2026-11-06: card written by the orchestrator.
- 2026-11-06: started. Read the card, `docs/agent-hooks.md` (six contract items), both hooks reviews, the OpenCode
  review major 1, the Results of `p4-contracts-r2`, `p4-opencode-source` and `p4-live-codex`.
- 2026-11-06 ~18:50: the orchestrator added items 8 (tray starts Copilot CLI, VS Code chat, OpenCode) and 9
  (`agent-watch.rs` rows for `vscode` and `opencode`); written into Deliverables.
- 2026-11-06: `command_line_contains` needs `NtQueryInformationProcess`, which is behind the `windows` feature
  `Wdk_System_Threading` (not on; the design's "no feature" line missed it). Plan: add that one feature to
  `crates/wakeshot-windows/Cargo.toml` (house way, as `Wdk_System_SystemInformation`); a feature is not in
  `Cargo.lock`, so `--locked` still holds. Reason: review major 3's runtime check (design contract 2).
- 2026-11-06: written (not yet built): `process` (parent, `image_path`, `command_line_contains`,
  `current_above_medium`, `harden_dll_search`, tests), `store` (`CREATE_NEW`, random part, 3 names, `temp_target`,
  `local_app_data` public, tests), core `HooksAdded` event, notice, answer, words and tests, `hooks.rs` step and fake,
  `sources/hooked.rs` stub, `agent_hooks.rs` owner, CLI `Hook` parse, dispatch, stub and `tests/hook.rs`.
- 2026-11-06: item 6 written: `names_db_file` takes only `<db>` and `<db>-wal`; tests
  `only_a_notice_of_the_database_or_its_wal_runs_a_query` (agents) and
  `a_query_that_rewrites_the_shm_runs_once_not_in_a_loop` (thread, real notices, the fake read rewrites the `-shm`).
  Workspace builds with all targets. The changed tests pass.
- 2026-11-06: broken on purpose, then put back (byte compare): the old `-shm` rule made both item 6 tests fail (the
  thread test counted 3 reads in 3 s, the loop); `CREATE_ALWAYS` made both temp name tests fail (the planted hard
  link's target was overwritten with the new bytes).
- 2026-11-06: items 8 and 9 written (`tray_mode.rs` starts Copilot CLI, VS Code chat and OpenCode with a test of
  each home rule; `agent-watch.rs` rows `vscode`, `opencode`). Docs: architecture sections 2, 3, 6, 7 (item 7 and
  the tray lines), 12; `agent-hooks.md` "Built" note; `research.md` "Contracts round 3 checks";
  `research/opencode-signals.md` one line. Ignored Explorer tests run once: the nested Medium run says
  `above Medium false`. Next: whole-workspace checks.
- 2026-11-06 15:39: first check run: fmt, x64 clippy (both), x64 release clean; `wakeshot_core` 708 passed, 1 failed
  (`wording_snapshot`: my own expected string had lost its line-continuation backslash in a shell edit; fixed). The
  ARM64 steps never reached cargo (my check script passed the argument list as one array). Script fixed; full rerun.
- 2026-11-06 15:42: full rerun, every check clean (Result). No process left from the build folder; build folder
  deleted.

## Result

Status: done, 2026-11-06. Nothing here ran against a real agent.

**What was built** (by item; the design's "Contract changes the build needs", six items, plus 6 to 9 of this card):

1. CLI: `Command::Hook { agent, event, values }` (`command.rs`), its line in `main.rs`, the parse in `args.rs` (checked
   first, so any words after `hook` give `Hook`, never `Help` or `Bad`; not in the help), a stub `hook.rs` (`Owner:
   p4-cursor-source`) that prints `{}` and exits 0, and `tests/hook.rs` (same owner) on the built exe.
2. `process`: `ProcessEntry.parent`; `image_path`, `command_line_contains` (the text is searched and dropped inside),
   `current_above_medium`, `harden_dll_search`. One `windows` feature added: `Wdk_System_Threading` (Log).
3. Core: `Event::HooksAdded { agent }`, `Notice::HooksAdded { agent }`, the answer (`Notify` for each event; it waits
   while the user is away, like every notice), the design's words in `text.rs` (quiet, info, no click).
4. `hooks.rs`: `HookEnv::remove_agent_hooks`, called right after `remove_notices`; `RealHookEnv` body an empty stub;
   the fake records it and five order tests now include it.
5. `sources/hooked.rs` stub (`Owner: p4-cursor-source`, in the `sources` module list); `agent_hooks.rs` owner now
   `p4-agent-hooks-install`.
6. Design item 6: `write_atomic` temp files `<name>.<pid>.<n>.<16 hex>.tmp`, made with `CREATE_NEW`, three names,
   then `ERROR_FILE_EXISTS`; no delete added. `store::temp_target` reads the shape for the prune.
7. Card item 6 (OpenCode review major 1): a `Query` runs on a notice only for `<db>` or `<db>-wal`, never `-shm`.
8. Card item 7: architecture section 7's Codex paragraph: the "can outlive its session" mark is only for the fallback.
9. Card item 8: the tray starts Copilot CLI (`COPILOT_HOME`, else `.copilot`; empty counts as unset), VS Code chat
   (`sources::vscode::installs`) and OpenCode (`opencode_db`), one log line each, no path in it.
10. Card item 9: `agent-watch.rs` rows `vscode` and `opencode`, and the help text.

**Shapes that differ from the design's list** (written into `docs/agent-hooks.md`, "Built"):

- `image_path(ProcessRef) -> Result<Option<PathBuf>, WinError>` and `command_line_contains(ProcessRef, &str) ->
  Result<bool, WinError>`: a `ProcessRef`, not a pid, so a reused pid never answers for another process.
- `remove_agent_hooks(&mut self)` returns nothing: the env logs one line per agent itself and never fails the hook.
- Added beyond the list: `store::local_app_data()` (public; the CLI needs `%LocalAppData%\cursor-agent\` and calls
  Windows only through `wakeshot-windows`), `store::temp_target`, `tests/hook.rs`.

**Choices** (mine), each with when to change it:

1. A `ProcessRef` for the two new process calls. Change it when a caller has only a pid (none does: the list gives
   each start).
2. The random part of a temp name from `std::hash::RandomState` (system-seeded keys), no new feature. `CREATE_NEW`
   already makes a guessed name harmless; the random part only stops a blocking guess. Change it when a stronger
   source is wanted: `BCryptGenRandom` (`Win32_Security_Cryptography`).
3. Any `hook` shape but the two known ones gives empty words (does nothing), not the plain form. Change it when a later
   version adds an argument: give it its own form then.
4. The `HooksAdded` words as the design gives them; quiet, since it is news and comes at a tray start.
5. The input parse and the state file text belong in `sources::hooked` (core), as its module doc says: the CLI has no
   JSON crate, and a `serde_json` line in `wakeshot-cli` would change `Cargo.lock` (read there).

**Files changed:** `crates/wakeshot-cli/src/{command,args,main}.rs`, `crates/wakeshot-cli/src/hook.rs` (new),
`crates/wakeshot-cli/tests/hook.rs` (new); `crates/wakeshot-core/src/{event,view,core,text,sources}.rs`,
`crates/wakeshot-core/src/sources/hooked.rs` (new), `crates/wakeshot-core/src/scenarios/lid.rs` (tests);
`crates/wakeshot-windows/Cargo.toml` (one feature), `crates/wakeshot-windows/src/{process,store,hooks,agent_hooks,
sqlite}.rs` (`sqlite.rs`: one test field), `src/watch/{agents,thread}.rs`, `src/app/{tray_mode,io_thread}.rs`,
`examples/{agent-watch,ux_all,ux_notices}.rs`; `docs/architecture.md` (sections 2, 3, 4, 6, 7, 12),
`docs/agent-hooks.md` (contract section), `docs/research.md` ("Contracts round 3 checks"),
`docs/research/opencode-signals.md` (one line), this card. `Cargo.lock`: no change.

**New tests:** process `the_public_list_names_a_childs_parent`, `image_path_names_the_exe_of_that_very_process`,
`command_line_contains_searches_that_very_process`, `the_command_line_reads_with_query_limited_rights_alone`,
`current_above_medium_reads_this_processs_integrity`, `harden_dll_search_keeps_a_dll_in_the_working_folder_out`
(with a child role), and the nested Explorer test now asserts `current_above_medium` is false at Medium (run with
`--ignored`: passed). Store `temp_names_have_one_shape_and_a_random_part`,
`a_planted_hard_link_at_a_temp_name_is_never_written_through`, `a_write_whose_names_all_exist_fails_and_deletes_nothing`.
Watch `only_a_notice_of_the_database_or_its_wal_runs_a_query`, `a_query_that_rewrites_the_shm_runs_once_not_in_a_loop`
(real notices). Core `each_hooks_added_gives_its_notice_and_nothing_else`, `hooks_added_while_locked_waits_for_the_unlock`,
`HooksAdded` in the proptest ops and in every `text.rs` table. CLI `the_hook_form_and_its_checked_values`,
`every_hook_shape_is_a_hook_never_help_or_an_error`, `every_hook_form_answers_an_empty_object_with_exit_0`. Tray
`the_tray_watches_copilot_cli_vs_code_chat_and_opencode_where_each_writes`. Broken on purpose: the old `-shm` rule and
`CREATE_ALWAYS`, each failed its new tests (Log).

**Checks** (own target folder, every one `--locked`, no `RUSTFLAGS`, final code, 2026-11-06 15:39 to 15:42):

- `cargo fmt --all --check`: clean.
- `x86_64-pc-windows-msvc`: clippy `--workspace --all-targets -D warnings` clean; without `--all-targets` clean;
  release build done.
- `aarch64-pc-windows-msvc` (Build Tools `VC\Tools\Llvm\x64\bin` on that command's PATH only): clippy with and
  without `--all-targets` clean; release build done.
- `cargo test --workspace --locked` (x64 host): 1622 passed, 0 failed, 58 ignored. Core unit 709, `claude_status` 11,
  `claude_transcript` 25, `codex_source` 22, `copilot_source` 42, `opencode_source` 15, `vscode_source` 31;
  `wakeshot-windows` 668; CLI 77, `exe` 3, `hook` 1, `run` 5; tray 8, `recovery` 5. The Codex, OpenCode and VS Code
  tests all pass.

**For the build cards:**

- `p4-cursor-source`: put the stdin parse and the state file JSON in `sources::hooked` (the CLI has no `serde`); the
  walk has `ProcessEntry.parent` and `start`, so pass each candidate as a `ProcessRef` to `image_path`; the runtime
  transport check is `command_line_contains(parent, "\"conversation_id\"")`; the prune matches temp names with
  `store::temp_target`; `%LocalAppData%` is `store::local_app_data()`. `tests/hook.rs` is yours to extend.
- `p4-agent-hooks-install`: fill `RealHookEnv::remove_agent_hooks` in `hooks.rs` (it logs its own lines; the fake
  and the order tests already have the step); `text.rs` words are done; post `Event::HooksAdded { agent }` with the
  label from your table.

**Open items:** Found outside scope 2 to 5. The live cards: the new tray sources through the app; the crash leftover
case for OpenCode.
