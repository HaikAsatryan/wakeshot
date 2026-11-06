# Agent hooks install: add, keep and remove Wakeshot's plugin folders

Phase: 4. Status: done (2026-11-06). Needs Haik: no.
Agent: `wakeshot-worker`. Size: about 180K tokens. If the card runs past about 210K, stop after a finished part with
its tests, and list the rest in the Result.
Runs after: `p4-contracts-r3`, the hooks security review verify (`docs/reviews/p4-hooks-design-security-verify.md`).
Parallel with: `p4-cursor-source` (owns `sources/hooked.rs`, `sources/cursor.rs`, `wakeshot-cli/src/hook.rs`), other
source and live cards that own different files; at most four Rust cards at once.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-agent-hooks-install\target`; delete it when done.
Real laptop: tests write only under temp folders. Never write into the real `%USERPROFILE%\.cursor` or `.gemini`
folders (the live card does that). No power settings, no screen.

## Goal

The install side of `docs/agent-hooks.md`: when the tray finds Cursor (and later Antigravity), it adds Wakeshot's own
plugin folder once, says so once, never fights the user, and the uninstall removes exactly what Wakeshot wrote.

## Read first

- `CLAUDE.md` (Checks, cargo path, elevation note), `docs/architecture.md` sections 2, 6 and 12, the uninstall and
  hooks parts.
- `docs/agent-hooks.md`, all of it (it is the spec): "Adding", "Removing", "Failure cases", the risk list.
- `docs/reviews/p4-hooks-design-security.md` and `docs/reviews/p4-hooks-design-security-verify.md`.
- `docs/tasks/p4-hooks-design.md` Result ("Build cards" row for this card, "Review fixes").
- `docs/tasks/p4-contracts-r3.md` Result; `crates/wakeshot-windows/src/hooks.rs` and `docs/tasks/p6-hooks.md`.

## Deliverables

1. `crates/wakeshot-windows/src/agent_hooks.rs`: the fixed file set and marker keys, the add, rewrite and remove
   table, reparse point checks, the staging folder with fail-if-exists and its cleanup, the `.wakeshot-removed`
   rename, the record (`agent-hooks.json`), the Medium-only rule for the tray, `hook <agent> remove`, as the design
   says. Cursor's plugin files now; leave one table entry point so `p4-antigravity-source` adds its own.
2. The uninstall step and its fake in `crates/wakeshot-windows/src/hooks.rs` (with the Medium copy when elevated).
3. The call in `crates/wakeshot-windows/src/app/startup.rs` that posts `Event::HooksAdded`; the notice words in
   `text.rs` if `p4-contracts-r3` left them to you.
4. Tests on temp folders: a fresh add, a second start adds nothing, a folder the user changed is never touched, a
   folder the user removed is never added back, a junction or link in any path refuses the write, a crafted record
   or marker deletes nothing outside the fixed file set, uninstall removes exactly the fixed set, an elevated run
   writes nothing in user folders (run the deny case under a Medium token: `CLAUDE.md` elevation note).
5. Rows for `agents\` and `agent-hooks.json` in `docs/architecture.md` section 6, and "Built" notes in
   `docs/agent-hooks.md` (re-read right before each edit; other cards add lines at the same time).

## Acceptance checks

- `cargo fmt` on your own files; `cargo clippy -p wakeshot-core -p wakeshot-windows -p wakeshot-tray --all-targets
  --locked -- -D warnings` and the same without `--all-targets`; `cargo test -p wakeshot-core -p wakeshot-windows -p
  wakeshot-tray --locked`. Record each result. A failure in another card's file is reported, never fixed. The
  orchestrator runs the whole-workspace checks.
- Break two rules on purpose (the link check and the fixed file set) and show the tests catch them.

## Out of scope

- The hook command, the state files and the sources (`p4-cursor-source`).
- Registering the source in `crates/wakeshot-windows/src/app/tray_mode.rs`, any real plugin folder, any live run
  (`p4-live-cursor`).
- Contracts, module lists, `Cargo` files: stop and ask the orchestrator if you need a change.

## Rules

- Never run git, not even read-only (`git status`, `git diff`, `git --version`).
- Never read token files or any app's secrets. Never write outside temp folders in tests.
- Kill only processes you started, by PID. Never set `RUSTFLAGS`. Cargo may not be on PATH:
  `%USERPROFILE%\.cargo\bin\cargo.exe`.
- Facts only from code or docs you read; mark guesses. Plain short English, no em dashes, no emojis. Write Log lines
  as you go.

## Questions for Haik

## Found outside scope

1. `crates/wakeshot-windows/src/app/tray_mode.rs` (not this card's): nothing calls `startup::add_agent_hooks` yet.
   One line in `serve`, after the window's `sink` exists, for example right after `Updater::start`:
   `let _agent_hooks = startup::add_agent_hooks(boot, Arc::clone(&sink), Arc::clone(log));` (it refuses a dry run by
   itself; the handle need not be joined). For `p4-live-cursor`, with the source's line.
2. `crates/wakeshot-cli/src/hook.rs` (`p4-cursor-source`): `hook <agent> remove` must call
   `wakeshot_windows::agent_hooks::remove_command(agent, log)` (it checks the token itself and ignores an agent it does
   not know), then print `{}` and exit 0 as every hook form does; the runtime transport check calls the same function.
   Until then an elevated uninstall's Medium copy (`wakeshot.exe hook cursor remove`) removes nothing; only a Medium
   uninstall removes in-process.
3. The design's table makes a Cursor that was uninstalled (`.cursor` deleted) and installed again look like "the user
   removed the folder": the record still says `added` and the folder is missing, so the mark turns `removed`, and the
   hooks never come back until Wakeshot is reinstalled or the record deleted. Built as the table says (the safe side
   of "never fight the user"); a decision for Haik's review in the Result.

## Log

- 2026-11-06: card written by the orchestrator from the hooks design.
- 2026-11-06: worker started. Read the card, `agent-hooks.md`, both hooks reviews, the design card Result and the
  `p4-contracts-r3` Result. Reading the code next.
- 2026-11-06: read `hooks.rs`, `store.rs`, `process.rs`, `app/startup.rs`, `tray_mode.rs` (read only). Plan: the
  install logic in `agent_hooks.rs` over a `Folders` value (profile, `%LocalAppData%`) so tests use temp folders; the
  token level and the Medium copy passed in, so tests pick them. `startup::add_agent_hooks` is the tray's entry; it
  needs one call line in `tray_mode::serve`, which this card does not own (for `p4-live-cursor`). `hooks::Places`
  gains an `agents` field (only `hooks.rs` builds it), so the uninstall test never reads the real profile.
- 2026-11-06: written and building (no tests yet): `agent_hooks.rs` (table, look, add with staging, rewrite, staging
  cleanup, removal with the `.wakeshot-removed` rename, record, Medium rule, `on_uninstall` with the Medium copy,
  `remove_command` for the CLI), `startup::add_agent_hooks`, `RealHookEnv::remove_agent_hooks`. Tests next.
- 2026-11-06: tests written: 20 in `agent_hooks` (2 more ignored: the nested Medium run and its role), 2 in
  `startup`, 1 in `hooks`; all pass. The ignored Medium run passed once: under Explorer's token `above Medium
  Ok(false)`, the folder added; this runner (High) adds nothing. Seen: a file held open without delete sharing fails
  `DeleteFileW` with 32, and the folder's rename then fails with 5. Next: the two broken rules, then the checks.
- 2026-11-06: broken on purpose, then put back (byte compare, script in the scratchpad). Link check: first by
  dropping the reparse arm of `kind`, which no test caught as an unsafe write, since std already does not report a
  junction as a folder (only a log text differed); then by looking through links (`fs::metadata` for
  `fs::symlink_metadata`): `a_link_anywhere_on_the_path_stops_every_write_and_delete` failed at `.cursor` ("nothing
  changed through the link"). Fixed file set: the plugin folder deleted with `remove_dir_all` instead of by the fixed
  names: `a_crafted_marker_or_record_deletes_nothing_outside_the_fixed_set` and
  `uninstall_renames_a_folder_it_cannot_empty` failed. Both restored; all pass again.
- 2026-11-06: checks run (Result): rustfmt on my three files, clippy both ways, tests of core, windows and tray, all
  clean. Docs written: `architecture.md` section 6 rows, `agent-hooks.md` "Built" note, `research.md` section. No
  process left from the build folder; build folder and scratch scripts deleted; no `wakeshot-agent-hooks-*` temp
  folder left.

## Result

Status: done, 2026-11-06. Nothing touched a real agent folder; every test ran on temp folders.

**What was built:**

1. `crates/wakeshot-windows/src/agent_hooks.rs` (about 1,000 lines of code, 900 of tests): the agent table
   (`AGENTS`, one entry for Cursor: parents `.cursor\plugins\local`, fixed folders `.cursor-plugin` and `hooks`, fixed
   files by key `plugin` and `hooks`, the marker `wakeshot-hooks.json`), the hook file contents (single-quoted exe with
   `'` doubled; U+2018 to U+201B, control characters and non-Unicode paths refused), the marker with FNV-1a 64 hashes
   by fixed key, `look` (by fixed names only; an extra name or a part that is not plain makes the folder "changed"; a
   file counts as Wakeshot's when it matches the marker or this version's bytes), the add through
   `.wakeshot-new-<pid>` made with fail-if-exists and renamed into place, the rewrite with `store::write_atomic` (files
   first, marker last), the staging cleanup at start and uninstall, the removal by fixed names with the
   `.wakeshot-removed` rename, the record `agent-hooks.json`, the reparse check on every part, the Medium-only rule,
   `on_uninstall` with the Medium copy (`medium_copy`: `wakeshot.exe hook <agent> remove` through
   `spawn_outside_tree`, 5 s wait), `remove_command` for the CLI, `installed_hook_exe` (only Velopack's `current`
   folder).
2. `hooks.rs`: `RealHookEnv::remove_agent_hooks` calls `agent_hooks::on_uninstall` with the real level and copy (tests
   swap both); `Places` gains `agents` (only `hooks.rs` builds it), so the rig's uninstall never reads the real
   profile.
3. `app/startup.rs`: `add_agent_hooks(boot, sink, log)`, a thread of its own that runs `at_start` for an installed,
   not dry-run tray and posts `Event::HooksAdded { agent }` per folder added. The notice words were done by
   `p4-contracts-r3`.
4. Docs: `architecture.md` section 6 rows for `agents\` and `agent-hooks.json`; `agent-hooks.md` "Built" note after
   "Removing"; `research.md` "Agent hooks install checks".

**Entry point for `p4-antigravity-source`:** one more `Agent` in `AGENTS` (`agent_hooks.rs`): parents
`[".gemini", "config", "plugins"]`, `agent_made: 2` (Wakeshot never makes `config`), `folders: &[]`, its fixed files,
a `present` function (`agy.exe` in its two folders, no `ANTIGRAVITY_GEMINI_DIR`) and a `contents` function that
returns `None` outside its path allow list. The rest (table, marker, staging, removal, record) needs no change.

**Choices** (mine), each with when to change it:

1. The tray's work runs on a short thread of its own, not the IO thread, whose queue takes only core commands (and
   `io_thread.rs` is not this card's). Change it when the IO queue takes jobs.
2. A fixed file is Wakeshot's when it matches the marker or this version's bytes, so a stop mid-rewrite never disowns
   the folder; a `write_atomic` temp file of a fixed name is a leftover, not an extra. Change it when a version writes
   files whose content depends on more than the exe path.
3. Uninstall keeps the marker while a fixed file stays, and removes empty fixed subfolders anyway. Change it when the
   README wants the renamed folder to hold only the user's files.
4. A record that does not read as version 1 adds nothing and is never rewritten (never fight the user when unsure).
   Change it when a later version needs to migrate it.
5. An elevated uninstall starts a copy only for an agent whose `local` folder exists. Change it when an agent's
   folders can sit elsewhere.
6. Log lines name folders from the profile (`.cursor\plugins\local\wakeshot`), never the user's path.

**Tests** (new: 20 in `agent_hooks` and 2 ignored there, 2 in `startup`, 1 in `hooks`): a fresh add once, and a second
start adds nothing; no `.cursor`, nothing; a removed folder never comes back (until the record is deleted); seven
kinds of user change never touched; older content rewritten with no notice; a stop mid-rewrite; six markers and a file
"not ours"; a junction at `.cursor`, `plugins`, `local`, `wakeshot`, `.cursor-plugin`, `hooks` and a staging folder,
a file link and a hard link: nothing changes outside; a crafted marker naming a victim file, extra files, crafted
records; uninstall removes exactly the fixed set; the rename and its failure at a second uninstall; a held-open file;
the staging cleanup; fail-if-exists staging; above Medium and an unknown level touch nothing, and the uninstall hands
off; the real token decides; the nested Medium run (ignored, run once: passed); the installed path rule; quoting;
FNV-1a vectors; `HooksAdded` posted once per folder, none for a dev build, above Medium or a dry run, a lost post
logged; the real uninstall at Medium removes, above Medium hands off.

**Broken on purpose** (Log): the link check (look through links) failed the link test at `.cursor`; the fixed file
set (a recursive delete) failed the crafted-marker and rename tests. Both put back, byte equal.

**Checks** (own build folder, `--locked`, no `RUSTFLAGS`, final code, 2026-11-06):

- `rustfmt --edition 2024 --check` on `agent_hooks.rs`, `hooks.rs`, `app/startup.rs`: clean.
- `cargo clippy -p wakeshot-core -p wakeshot-windows -p wakeshot-tray --all-targets --locked -- -D warnings`: clean;
  the same without `--all-targets`: clean.
- `cargo test -p wakeshot-core -p wakeshot-windows -p wakeshot-tray --locked`: all passed. Core 730 (1 ignored),
  `claude_status` 11, `claude_transcript` 25, `codex_source` 22, `copilot_source` 46, `cursor_source` 8,
  `opencode_source` 15, `vscode_source` 31; `wakeshot-windows` 694 (28 ignored); tray 8, `recovery` 5 (2 ignored);
  e2e and smoke ignored as before.
- `cargo test -p wakeshot-windows --locked --lib -- --ignored
  agent_hooks::tests::a_medium_start_writes_and_an_elevated_one_does_not`: passed once.

**Files changed:** `crates/wakeshot-windows/src/agent_hooks.rs`, `crates/wakeshot-windows/src/hooks.rs`,
`crates/wakeshot-windows/src/app/startup.rs`, `docs/architecture.md` (section 6, two rows), `docs/agent-hooks.md`
("Built" note), `docs/research.md` (new last section), this card. No contract, module list or `Cargo` change.

**Open items:** Found outside scope 1 (the `tray_mode` call line) and 2 (the CLI's `remove` form); until both land, no
installed tray adds hooks, and an elevated uninstall removes none. Live checks for `p4-live-cursor`: the installed tray
from `current` adds the folder and the notice shows; the editor skips the dot-named folders.

**For Haik's review:** a Cursor uninstalled and installed again is taken as "the user removed the hooks" (Found outside
scope 3), as the design's table says. Default: keep it. The other way: forget the mark when `.cursor` is gone at a
start, so a reinstalled Cursor gets the hooks again.
