# Antigravity source: its event table and plugin folder

Phase: 4. Status: done, 2026-11-06 (new 2026-11-06, Haik:
Antigravity in v1 if a free test exists; `p4-antigravity-research` says it does). Needs Haik: only if step 1 finds
the payload on the command line.
Agent: `wakeshot-worker`. Size: about 100K tokens.
Runs after: `p4-cursor-source`, `p4-agent-hooks-install`. Parallel with: cards that own different files; at most four
Rust cards at once.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-antigravity-source\target`; downloads in their own empty folder,
as data only. Delete the target folder when done.
Real laptop: Antigravity may be installed by `p4-live-harness`; you do not run it or add its plugin (the live card
does). No power settings, no screen.

## Goal

Antigravity (CLI and IDE) on the shared hooked-source code: its event table in `sources/antigravity.rs`, its plugin
folder in the install table, as `docs/agent-hooks.md` "Antigravity" designs them.

## Read first

- `docs/agent-hooks.md` (all; "Antigravity" is your part), `docs/research/antigravity-signals.md`.
- `docs/tasks/p4-cursor-source.md` and `docs/tasks/p4-agent-hooks-install.md` Results: the shared code and the one
  entry point each left for you.
- `CLAUDE.md` (Checks, cargo path, elevation note).

## Deliverables

1. **First: the payload form** of Antigravity's hooks (stdin, temp file or command line), read in its current
   Windows build as data (design "Privacy gate"). If it is the command line, stop, write it under "Questions for
   Haik" with the default "does not ship", and tell the orchestrator.
2. `crates/wakeshot-core/src/sources/antigravity.rs`: the event table (`PreInvocation`, `Stop`, `fullyIdle`, the
   `state` field), the `AgentSource`, fixed labels, on `sources/hooked.rs`.
3. Antigravity's plugin files in the install table (`crates/wakeshot-windows/src/agent_hooks.rs`, through the entry
   point `p4-agent-hooks-install` left), and the `hook` command's agent name if `hook.rs` needs one line for it.
4. Tests in the shape of Cursor's: event rules, the privacy marker test, the fixed file set, the liveness pairing
   with `agy.exe`.
5. Wiring for both hooked sources (orchestrator, 2026-11-06 ~18:50, so the two live cards can run in parallel):
   start the Cursor and Antigravity sources in `crates/wakeshot-windows/src/app/tray_mode.rs` (as `p4-contracts-r3`
   did for Copilot, VS Code and OpenCode, each with a unit test), and add `cursor` and `antigravity` rows to
   `SOURCES` in `crates/wakeshot-windows/examples/agent-watch.rs`. This wins over "Out of scope" below.
6. The one `startup::add_agent_hooks` call in `tray_mode::serve` (`p4-agent-hooks-install` Result, "Not wired yet"),
   so the tray adds the plugin folders, with a test.
7. Orchestrator decision, 2026-11-06 ~20:30, for Haik's review (easy to undo): a whole agent folder gone (for example
   `%USERPROFILE%\.cursor` deleted with Cursor, then made again by a reinstall) is the agent going away, not the user
   removing Wakeshot's hooks, so the record forgets that agent and the hooks are added again when it is found.
   Only Wakeshot's own folder gone, with the agent folder still there, stays "the user removed them" (never added
   back). In `agent_hooks.rs`, with tests for both cases. Today's code treats both as "removed by the user", so a
   Cursor reinstall would lose Wakeshot's hooks for good.

## Acceptance checks

- `cargo fmt` on your own files; `cargo clippy -p wakeshot-core -p wakeshot-windows -p wakeshot-cli --all-targets
  --locked -- -D warnings` and the same without `--all-targets`; `cargo test -p wakeshot-core -p wakeshot-windows -p
  wakeshot-cli --locked`. Record each result. The orchestrator runs the whole-workspace checks.

## Out of scope

- Registering the source in `crates/wakeshot-windows/src/app/tray_mode.rs` and any live run
  (`p4-live-antigravity`). Contracts, module lists, `Cargo` files.

## Rules

- Never run git, not even read-only (`git status`, `git diff`, `git --version`).
- Never read token files or any app's secrets. Prompt text is never logged or stored. Downloads are data, never run.
- Kill only processes you started, by PID. Never set `RUSTFLAGS`. Cargo may not be on PATH:
  `%USERPROFILE%\.cargo\bin\cargo.exe`.
- Facts only from code or docs you read, with the version; mark guesses. Plain short English, no em dashes, no
  emojis. Write Log lines as you go.

## Questions for Haik

None. Step 1 found the input on stdin, so the "does not ship" question does not arise. Item 7 is the orchestrator's
decision; it is listed for Haik's review in the Result.

## Found outside scope

1. The Antigravity App (2.29.0, "Antigravity 2.0") also reads `~\.gemini\config\` (research), but it is not installed
   here and was not read, so the Privacy gate is checked for the CLI and the IDE only. Likely the same Go engine
   (not verified). Suggest: one short read of the App's Windows build, as data, before the plugin ships to users who
   have it; or `p4-live-antigravity` names it.
2. `agy` 1.5.2's plugin listing has no dot-name check (`research.md`, "Antigravity hook payload form"), so the design's
   `.wakeshot-removed` (uninstall) and `.wakeshot-new-*` (staging) may be listed as plugins. Staging is now safe
   enough (manifest last, and a leftover is removed at the next start). `.wakeshot-removed` loads only if it still
   holds a `plugin.json`, which happens only when the user changed it; then the hooks it runs are the user's own
   edit and name an exe the uninstall deleted (fail open). Not fixed; live test 7 checks it.
3. `examples/agent-watch.rs` `--raw`: a hooked state file fed whole prints the Claude status fields (all `-`); no
   private text, but no `state` either. Change it when a live card wants the hooked files' `state` in the raw log.
4. `docs/agent-hooks.md`, "Adding": the "missing | any | any | nothing" row now also forgets the mark (item 7); the
   row text itself is left as it was (shared doc, additions only), and the "Changed" note under "Antigravity" says
   it. Fold it into the table when the doc is next edited by its owner.

## Log

- 2026-11-06: card written by the orchestrator from the hooks design.
- 2026-11-06: worker started. Read the card, `CLAUDE.md`, plan "How we work", `docs/agent-hooks.md`,
  `research/antigravity-signals.md`, the Results of `p4-cursor-source` and `p4-agent-hooks-install`. Step 1 (the
  payload form, from the installed `agy` as data) first, before any code.
- 2026-11-06: step 1 done, by reading the installed `agy.exe` 1.5.2 as data (the CLI's and the IDE's copies have the
  same SHA-256; nothing run). The hook executor `jsonhook.executeCommandModeHook` runs `cmd /c <command>` through
  `os/exec.CommandContext`, adds `ANTIGRAVITY_CONVERSATION_ID=<id>` to the environment, and gives the payload as
  `Stdin` (a `bytes.Reader`), with stdout read and stderr in a `bytes.Buffer`. The IDE 2.6.0 runs no hooks itself:
  its JS only shows and edits the global `hooks.json`. So the input never reaches a command line: the Gate holds, the
  Antigravity plugin may ship, no question for Haik. Also found: both marshal calls set protojson `EmitUnpopulated`,
  and `fullyIdle` is a plain proto3 bool, so `Stop` always carries it, `false` included; the payload has no event
  name and no version field. Details in `docs/research.md`, "Antigravity hook payload form".
- 2026-11-06: core done: `sources/antigravity.rs` (`AGENT`: `conversationId`, idle flag `fullyIdle`, no turn, event
  or version field; `PreInvocation` writes working and prunes, `Stop` is `StopIfIdle`; `agy.exe` under
  `%LocalAppData%\agy\bin` is "Antigravity CLI", any other `agy.exe` is "Antigravity"), 5 synthetic fixtures with a
  README, `tests/antigravity_source.rs` (13 tests, all pass). Next: the plugin entry in `agent_hooks.rs` and item 7.
- 2026-11-06: read in `agy` too: `expandHome` expands only a leading `~/` (it compares the first two bytes with `~/`),
  so a `~` inside a path is left alone; the allow list still refuses it (real known folders never hold one).
- 2026-11-06: `agent_hooks.rs` done: `ANTIGRAVITY` in `AGENTS` (parents `.gemini\config\plugins`, `agent_made` 2,
  `plugin.json` and `hooks.json` in the plugin folder, `present` = `agy.exe` in one of its two folders and no
  `ANTIGRAVITY_GEMINI_DIR`, contents with the bare-path rule `cmd_bare`); item 7 in `start_one` (the agent's own
  folder gone: the mark is forgotten, one log line). 8 new tests (incl. a junction at each Antigravity part), the 20
  old ones unchanged; all pass. Tests build the Antigravity world under the temp folder's long name, since a
  CI temp folder can be a short name with `~`. Next: the hook command line, then the tray wiring.
- 2026-11-06: `hook.rs`: `antigravity::AGENT` in `AGENTS`; the old test that used `antigravity PreInvocation` as an
  unknown pair now uses `antigravity SessionStart` (an event Wakeshot does not hook), and the fake's transport check
  accepts each agent's own key. 7 new in-process tests; `tests/hook.rs` 1 new exe test (2 events x 11 inputs) and 1
  new ignored chain test (a `cmd.exe` copy named `agy.exe` runs `cmd /c "<exe> hook antigravity <event>"` with the
  input on stdin), run here with `--ignored`: passed. Also checked by a raw command line that `cmd /c "\"<path>\"
  ..."`, the form Go would give a quoted path, fails: so the bare-path rule stands.
- 2026-11-06: tray wiring done: `tray_mode::hooked_sources` (Cursor and Antigravity, beside `sources`, so they run
  even when the profile folder is unknown) and `tray_mode::start_agent_hooks` (the one `startup::add_agent_hooks`
  call, in `serve` right after `Updater::start`), 2 tests; `agent-watch` rows `cursor` and `antigravity` (read
  `agents\<agent>` of `DataDir::real()`, so of `WAKESHOT_DATA_DIR` in a debug build; read only).
- 2026-11-06: read in `agy`: `isValidPluginDirName` refuses only empty, `.`, `..` and names with `/` or `\`, and
  `dirNames` (the plugin listing) has no dot check, so `.wakeshot-new-*` and `.wakeshot-removed` are likely listed.
  Antigravity's fixed files now put `plugin.json` last, so a staging folder is a plugin only with its last file; a
  test covers a stop before it. Rest in "Found outside scope".
- 2026-11-06: broken on purpose, then put back (byte equal, script in the scratchpad): a space in the bare-path allow
  list failed `a_path_cmd_would_split_gets_no_antigravity_hooks`; item 7's forget turned off (`&& false`) failed
  `an_agent_whose_own_folder_went_gets_its_hooks_again_when_it_is_back`.
- 2026-11-06: final checks on the final code (Result), docs written, build folder and scratch scripts deleted.

## Result

Status: done, 2026-11-06. Nothing ran Antigravity or added a plugin; no power settings; no git. The real
`%LocalAppData%\Wakeshot\agents`, `agent-hooks.json`, `.gemini\config\plugins\wakeshot` and
`.cursor\plugins\local\wakeshot` stayed absent through every run.

**1. Payload form** (`agy` 1.5.2, the CLI's and the IDE 2.6.0's copies are one binary, read as data): stdin. The hook
executor runs `cmd /c <command>` through Go's `os/exec`, the payload as `Stdin`, the chat id also in
`ANTIGRAVITY_CONVERSATION_ID`; the IDE's JS runs no hook. **The Antigravity plugin may ship.** Also read: `fullyIdle`
is always in `Stop` (protojson `EmitUnpopulated`); no event, turn or version field. `docs/research.md`, "Antigravity
hook payload form".

**What was built:**

1. `crates/wakeshot-core/src/sources/antigravity.rs`: `AGENT` (`conversationId`, idle flag `fullyIdle`;
   `PreInvocation` writes working and prunes, `Stop` deletes or keeps `background`; `agy.exe` under
   `%LocalAppData%\agy\bin` is "Antigravity CLI", any other `agy.exe` "Antigravity"), `LABEL`, `CLI_LABEL`,
   `source(data_root)`; 5 synthetic fixtures with a README.
2. `crates/wakeshot-windows/src/agent_hooks.rs`: `ANTIGRAVITY` in `AGENTS` (`.gemini\config\plugins\wakeshot\`,
   `agent_made` 2, `hooks.json` then `plugin.json`, presence by `agy.exe` in its two folders and no
   `ANTIGRAVITY_GEMINI_DIR`, the command `<exe> hook antigravity <event>` with a bare path only, `cmd_bare`); item 7
   in `start_one`.
3. `crates/wakeshot-cli/src/hook.rs`: `antigravity::AGENT` in `AGENTS` (and one old test's unknown pair changed).
4. `crates/wakeshot-windows/src/app/tray_mode.rs`: `hooked_sources` (both sources) and `start_agent_hooks` (the one
   `startup::add_agent_hooks` call, in `serve` after `Updater::start`). `examples/agent-watch.rs`: rows `cursor`,
   `antigravity`.
5. Docs: `agent-hooks.md` ("Built" and "Changed" under "Antigravity"), `architecture.md` (section 7 paragraph, the
   Cursor line's "not yet started" fixed, section 12 bullet), `research.md` (new last section).

**Tests** (new: core `antigravity_source` 13; `agent_hooks` 9; CLI `hook` unit 7; `tests/hook.rs` 1, plus 1 ignored
chain test run with `--ignored`; `tray_mode` 2): event rules and the idle flag, crafted ids, the privacy marker in
every field (core and CLI, files and log lines), the fixed file set and changed folders, a junction at each part, the
bare-path rule, the liveness pairing with the nearest `agy.exe` (pid and start), the elevated hand-off, the tray's
sources and its hook call, item 7 both ways. Broken on purpose (Log): the allow list and item 7; both caught, put back.

**Checks** (own build folder, `--locked`, no `RUSTFLAGS`, final code):

- `rustfmt --edition 2024 --check` on my 7 Rust files: clean.
- `cargo clippy -p wakeshot-core -p wakeshot-windows -p wakeshot-cli --all-targets --locked -- -D warnings`: clean;
  without `--all-targets`: clean.
- `cargo test -p wakeshot-core -p wakeshot-windows -p wakeshot-cli --locked`: all passed. Core 730 (1 ignored),
  `antigravity_source` 13, `claude_status` 11, `claude_transcript` 25, `codex_source` 22, `copilot_source` 48,
  `cursor_source` 8, `opencode_source` 15, `vscode_source` 33; `wakeshot-windows` 709 (28 ignored); CLI 99 (3
  ignored), `exe` 3, `hook` 5 (4 ignored; all 4 passed with `--ignored`), `run` 5.

**Choices** (mine), each with when to change it:

1. Bare-path rule kept as designed, now settled by code: `cmd` cannot read Go's `\"`. Change it when a user with a
   space in the profile path asks: then a short-name (8.3) path or a `.cmd` shim in Wakeshot's own folder.
2. Any `agy.exe` above the hook counts, labelled by folder. Change it when a live test finds an `agy.exe` that is not
   the agent's (a daemon) nearer than the one that ran the turn.
3. `hooks.json` before `plugin.json`. Change it when the live test shows `agy` skips dot-named folders.
4. Hooked sources are added beside `sources`, so they run without a profile folder. Change it when a source needs
   the profile.
5. The tray's hook thread is not joined. Change it when shutdown must wait for it.

**For Haik's review** (orchestrator decision, item 7, easy to undo): an agent's own folder gone at a tray start
(`.cursor`, `.gemini\config`) forgets that agent's mark, `added` or `removed`, so a reinstall gets the hooks and the
notice again; only Wakeshot's folder gone, the agent's still there, stays "removed for good". Gap: a reinstall
between two tray starts is not seen.

**Open items:** `p4-live-antigravity`: live test 7 (dropped folder loads and is enabled, dot names, `{}` lets `Stop`
through, Esc, the IDE hub as the ancestor, cost per call), real captures for the fixtures; Found outside scope 1 (the
App).
