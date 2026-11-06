# Cursor source: the hook command and the state files it writes

Phase: 4. Status: done (new 2026-11-06, Haik: Cursor in v1). Needs Haik: only if
step 1 finds the editor puts the payload on the command line (design Question 4).
Agent: `wakeshot-worker`. Size: about 210K tokens. If the card runs past about 230K, stop after a finished part with
its tests, and list the rest in the Result.
Runs after: `p4-contracts-r3`, the hooks security review verify (`docs/reviews/p4-hooks-design-security-verify.md`).
Parallel with: `p4-agent-hooks-install` (owns `agent_hooks.rs`, `hooks.rs`, `app/startup.rs`, `text.rs`), other
source and live cards that own different files; at most four Rust cards at once.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-cursor-source\target`; downloads in their own empty folder under
`<scratchpad>\p4-cursor-source\`, as data only. Delete the target folder when done.
Real laptop: Cursor may be installed by `p4-live-harness`; you do not run it or add any plugin (the live card does).
No power settings, no screen.

## Goal

The shared hooked-source code, Cursor's source and the `wakeshot.exe hook` command, as `docs/agent-hooks.md`
designs them after the security review, so a Cursor turn holds the PC while it runs and nothing of the prompt is
kept.

## Read first

- `CLAUDE.md` (Checks, cargo path, elevation note), `docs/architecture.md` sections 2, 6, 7 and 12.
- `docs/agent-hooks.md`, all of it (it is the spec), and its "Live test" list.
- `docs/reviews/p4-hooks-design-security.md` and `docs/reviews/p4-hooks-design-security-verify.md`.
- `docs/tasks/p4-hooks-design.md` Result ("Build cards" row for this card, "Review fixes").
- `docs/tasks/p4-contracts-r3.md` Result (what the contracts give you; its shapes win).

## Deliverables

1. **First, before any code: the editor's payload form** (review major 3, design live test 0). Read the Cursor
   editor's hook spawn code (the current Windows build, as data) and write down how the payload reaches the hook:
   stdin, a temp file, or the command line. The editor and the CLI load one plugin folder, and the transport is a
   runtime option in Cursor's code (verify review), so follow the design's privacy Gate as it reads after the verify:
   if the editor can use the command line, Wakeshot adds no Cursor plugin at all. Then write it under "Questions for
   Haik" (design Question 4), tell the orchestrator, and go on with the shared code only.
2. `crates/wakeshot-core/src/sources/hooked.rs`: the state file format, the id rule, the event rules, the parent walk
   over a process list, as the design says.
3. `crates/wakeshot-core/src/sources/cursor.rs`: Cursor's event table, the `AgentSource`, fixed labels.
4. `crates/wakeshot-cli/src/hook.rs`: the command (DLL search and panic hook first, the Medium hand-off when
   elevated, the prune rule), always `{}` and exit 0.
5. Tests and input fixtures as the design's build row lists: a marker-word privacy test (the word in every input
   field never reaches a state file, a log line or stdout), `{}` and exit 0 for every input and for a panic, a
   crafted chat id (traversal, reserved names, long names), a junction test under an elevated token (and the same
   at Medium; `CLAUDE.md` elevation note: run the deny case unelevated or it passes for the wrong reason).
6. Lines in `docs/agent-hooks.md` ("Built" notes) and `docs/architecture.md` section 7 (re-read right before each
   edit; other cards add lines at the same time).

## Acceptance checks

- `cargo fmt` on your own files; `cargo clippy -p wakeshot-core -p wakeshot-windows -p wakeshot-cli --all-targets
  --locked -- -D warnings` and the same without `--all-targets`; `cargo test -p wakeshot-core -p wakeshot-windows -p
  wakeshot-cli --locked`. Record each result. A failure in another card's file is reported, never fixed. The
  orchestrator runs the whole-workspace checks.
- Break two rules on purpose (the privacy rule and the exit code rule) and show the tests catch them.

## Out of scope

- Writing or removing plugin folders, the uninstall step, the notice (`p4-agent-hooks-install`).
- Registering the source in `crates/wakeshot-windows/src/app/tray_mode.rs` and any live run (`p4-live-cursor`).
- Antigravity's table (`p4-antigravity-source`, after this card).
- Contracts, module lists, `Cargo` files: stop and ask the orchestrator if you need a change.

## Rules

- Never run git, not even read-only (`git status`, `git diff`, `git --version`).
- Never read token files or any app's secrets. Prompt text is never logged or stored. Downloads are data, never run.
- Kill only processes you started, by PID. Never set `RUSTFLAGS`. Cargo may not be on PATH:
  `%USERPROFILE%\.cargo\bin\cargo.exe`.
- Facts only from code or docs you read, with the version; mark guesses. Plain short English, no em dashes, no
  emojis. Write Log lines as you go.

## Questions for Haik

None. Step 1 found a temp file in every editor path that runs a plugin hook, so design Question 4 does not arise.

## Found outside scope

1. `copilot_source.rs::every_live_fixture_is_listed_once` failed once in my package run while `p4-live-copilot` was
   adding fixtures; a re-run a minute later passed (48 tests). Its card's file; not touched.
2. The log folder has the same gap as `agents\` had: it sits in the data folder, so any elevated CLI command (an
   elevated `wakeshot run`, the guard if elevated) opens `logs\wakeshot.log` through whatever a Medium process put
   there. The hook avoids it (no log line above Medium); the other commands do not. Change it when the log writer
   refuses reparse points.
3. `agent_hooks::remove_command` logs "removes no hook folder" above Medium through the log it is given; the hook
   gives it the quiet log above Medium (the hook writes no log line there), so an elevated `hook cursor remove` that is not the Medium copy
   leaves no line. The uninstall's own side logs the copy's outcome (`medium_copy`), so nothing is lost there.

## Log

- 2026-11-06: card written by the orchestrator from the hooks design.
- 2026-11-06: worker started. Read the card, `CLAUDE.md`, plan "How we work", `docs/agent-hooks.md`. Step 1 (the
  editor's payload form) first, before any code.
- 2026-11-06: step 1 done, by reading the installed editor 3.27.12 and CLI 2026.11.05-86047df as data (nothing run).
  The editor runs plugin hooks only from its renderer through the extension host, and on Windows that path is a
  constant temp file (`windows_temp_file`), never the command line. The agent host daemon's executor defaults to the
  command line, but it is built in one place, with project hooks only, never plugin hooks. So the Gate holds: the
  Cursor plugin may ship. No question for Haik. Details in `docs/research.md`, "Cursor hook payload form".
- 2026-11-06: found while reading: Node quotes a `"` inside an argument as `\"` (checked with Node 26 spawning
  PowerShell `-c`), so the design's needle `"conversation_id"` (with quotes) would miss a here-string payload that
  Cursor's daemon passes through Node. The runtime check searches the bare key `conversation_id` instead.
- 2026-11-06: plan. Core `sources::hooked`: `Agent` table type, streaming four-field parse (`DeserializeSeed`, other
  keys skipped as `IgnoredAny`), id rules, event rules (incl. Antigravity's idle stop, so its card adds only a
  table), state and problem file text, walk over a list, checked values for the hand-off, and a generic
  `HookedSource`. `sources::cursor`: the table and `source()`. CLI `hook.rs`: real steps behind an `Env` trait so
  unit tests run in-process with fakes; exe tests for `{}`/exit 0, the panic and privacy; Explorer-dependent tests
  `#[ignore]` like the existing nested-token tests.
- 2026-11-06: core done: `sources/hooked.rs` (21 unit tests) and `sources/cursor.rs`, fixtures in
  `tests/fixtures/cursor/` (synthetic, from Cursor's field names), `tests/cursor_source.rs` (8 tests, marker privacy
  over every field of every fixture). All pass. Core tests build paths with `join`, no `C:\` literals (orchestrator
  note on `core-linux`). Next: the CLI `hook.rs`.
- 2026-11-06: CLI `hook.rs` done with 14 in-process tests (fake `Env`), all pass. The CLI has no `serde_json`, so its
  tests build inputs from field lists that a test checks against the fixtures. Next: exe tests in `tests/hook.rs`
  (answer and exit code for every input and a panic; junction tests).
- 2026-11-06: orchestrator: `p4-agent-hooks-install` is done; `hook <agent> remove` must call
  `agent_hooks::remove_command`. Wired through a new `Env::remove_hooks` (the fake records it, so no test touches the
  real profile): the `remove` form passes the agent word as given and reads no input; the blocked path (input on a
  command line) also calls it, at Medium only, as the design's "removes the agent's plugin". Tests added.
- 2026-11-06: the contracts card's exe test ran `hook cursor remove` with no data folder; at Medium that would now
  remove the real profile's plugin. Replaced by `hook nobody remove`, and every exe test sets `WAKESHOT_DATA_DIR`.
- 2026-11-06: exe tests in `tests/hook.rs`: 4 always-run (forms; 11 inputs x 3 events incl. 65 MiB and BOM, all read
  to the end; a panic; the marker) and 3 `#[ignore]` (need Explorer): the chain under a `Cursor.exe` (a copy of
  `cmd.exe`) writes and its stop deletes, through the Medium copy, which proves the copy gets the test's environment;
  an elevated hook writes nothing through a junction to a High-labeled folder; the Medium copy writes a plain folder
  and is denied (CreateFileW 5) through the junction. All pass; the real `%LocalAppData%\Wakeshot\agents` stayed absent.
- 2026-11-06: found and fixed in my own code: the log opened eagerly at Medium (an empty `wakeshot.log` for a hook with
  nothing to say); now `Lines` opens it at the first line. The first junction test ran prompt then stop and checked
  at the end, so a broken rule's write was deleted again by the stop: it now keeps a victim file in the target and
  checks after each event.
- 2026-11-06: rules broken on purpose, each put back (no `|| true` or `&& false` left, grep checked):
  1. Privacy: the turn id check off (`!is_turn_id(&turn) && false`). Failed: core
     `the_turn_id_is_short_and_plain_or_empty`, core `the_marker_in_any_field_never_reaches_a_file_or_a_log_text`
     (`turn: "QXV7 marker"` in the parse), CLI `the_marker_in_any_field_never_reaches_a_file_or_a_log_line` (the state
     file held `"generation":"QXV7 marker"`).
  2. Exit code: the panic hook without `process::exit(0)`. Failed: `a_panic_still_answers_...` (exit 101, want 0).
  3. Extra, the Medium rule (`medium = ... || true`): failed the in-process elevated test and the junction test (the
     elevated hook replaced the victim in the High-labeled target).

## Result

Status: done, 2026-11-06. Nothing ran Cursor or added a plugin; no power settings; no git.

**Step 1, the editor's payload form** (Cursor 3.27.12 and CLI 2026.11.05-86047df, read as data): the editor runs
plugin hooks from its renderer through the extension host, and on Windows that path is the constant
`windows_temp_file` (`%TEMP%\cursor-hook-payload-*.json`, piped by `Get-Content`, deleted after). The agent host
daemon's executor defaults to the command line but is built once, with project hooks only, never plugin hooks. Every
local CLI path sets the temp file. So the Gate holds and **the Cursor plugin may ship**. Details:
`docs/research.md`, "Cursor hook payload form".

**What was built:**

1. `crates/wakeshot-core/src/sources/hooked.rs`: the `Agent` table type; the streaming parse that keeps only the
   table's fields (others skipped as `IgnoredAny`, keys matched without being kept, duplicates refused, BOM skipped,
   errors kept as class and place only); the chat id rule (UUID shape, lowercased), the turn id rule, the version
   shown in a log only when version-shaped; the event rules (`Write`, `DeleteIfTurn`, `Delete`, and Antigravity's
   `StopIfIdle`); the state and problem files (`version` 1, `procStart` as a string, newer told apart, arrays
   refused); the walk (nearest match, at most 10 parents, a parent must be listed and not younger, `node.exe` only
   under its folder, compared part by part with case ignored); the checked values for the hand-off; `HookedSource`.
2. `crates/wakeshot-core/src/sources/cursor.rs`: `AGENT` (fields, three events, `Cursor.exe`, `node.exe` and
   `cursor-agent.exe` under `%LocalAppData%\cursor-agent`), `LABEL`, `CLI_LABEL`, `source(data_root)`.
3. `crates/wakeshot-cli/src/hook.rs`: DLL search limited and a panic hook (`{}`, exit 0) first; the steps behind an
   `Env` trait; parse, drain, walk, the runtime transport check, the Medium rule (above Medium: no file, no log line,
   hand-off to `spawn_outside_tree` with checked values, 2 s wait and deadline), the checked copy's re-checks, the
   mutex, the event's work, the prune; `hook <agent> remove` and the blocked path call
   `agent_hooks::remove_command` (orchestrator's wiring item).
4. Fixtures `crates/wakeshot-core/tests/fixtures/cursor/` (5 inputs, synthetic, from Cursor's field names; README).
5. Docs: `docs/agent-hooks.md` "Built" note under "How the source reads them"; `docs/architecture.md` section 7
   Cursor paragraph; `docs/research.md` "Cursor hook payload form" and "Cursor hook command checks".

**Files changed:** the five above (`hooked.rs`, `cursor.rs`, `hook.rs`, `tests/hook.rs` in the CLI, new
`crates/wakeshot-core/tests/cursor_source.rs` and the fixture folder), the three docs, this card. No contract, module
list or `Cargo` file.

**Tests:** core `hooked` 21 unit, `cursor_source` 8; CLI `hook` 17 unit (fake `Env`), `tests/hook.rs` 4 plus 3
`#[ignore]` (need Explorer, run here with `--ignored`: passed). They cover: the marker in every field of every
fixture (string, number, object, key) never in a file name, a file or a log line; `{}` and exit 0 for every form, 11
inputs x 3 events (65 MiB, BOM, bytes, deep nesting) and a panic; crafted chat ids (traversal, `CON`, `COM1`, 5,000
characters) giving only `problem.json`; the blocked path; the elevated hand-off with exact values; the copy's
re-checks and deadline; the prune; the junction under an elevated token and at Medium (a High-labeled target).

**Rules broken on purpose** (Log): the privacy rule (turn id check off: 3 tests failed) and the exit code rule (panic
hook without `exit(0)`: exit 101); extra, the Medium rule (2 tests failed). All put back.

**Checks** (own target folder, `--locked`, no `RUSTFLAGS`, final code, 2026-11-06):

- `rustfmt --check` on my five Rust files: clean.
- `cargo clippy -p wakeshot-core -p wakeshot-windows -p wakeshot-cli --all-targets --locked -- -D warnings`: clean;
  the same without `--all-targets`: clean.
- `cargo test -p wakeshot-core -p wakeshot-windows -p wakeshot-cli --locked`: all passed but one test of another card
  (`copilot_source` `every_live_fixture_is_listed_once`, mid-edit by `p4-live-copilot`; passed on re-run). Core unit
  730 passed (1 ignored), `cursor_source` 8, `wakeshot-windows` 695 (28 ignored); CLI unit 92 (3 ignored), `exe` 3,
  `hook` 4 (3 ignored, passed with `--ignored`), `run` 5.
- The real `%LocalAppData%\Wakeshot\agents` stayed absent through every run. Build folder deleted.

**Choices** (mine), each with when to change it:

1. Runtime check needle: the bare key `conversation_id` (Node escapes quotes). Change it when a Cursor build uses its
   `argv_base64` mode (a name only today, no builder found): base64 needs its own needles.
2. An elevated hook writes no log line (its log folder is swappable too). Change it when the log writer refuses
   reparse points.
3. A version reaches a log line only when it starts with a digit and is all of `0-9 A-Z a-z . -`, cut to 32. Change
   it when Cursor ships a version of another shape.
4. A newer file version is ignored silently, a bad file twice is no signal and no notice. Change it when a file
   Wakeshot did not write shows up in the folder in practice.
5. `Cursor.exe` matches by name alone, as the design says. Change it when a live test finds another `Cursor.exe`
   above a hook.
6. The Explorer-dependent exe tests are `#[ignore]`, as the house's nested-token tests are. Change it when CI has
   Explorer as its shell.
7. The panic test reads `WAKESHOT_TEST_HOOK_PANIC` in debug builds only; under the release `panic = "abort"` the hook
   runs before the abort (Rust docs), **not verified** by a test.

**Open items:** `p4-live-cursor`: the tray line (`Box::new(sources::cursor::source(data.root()))`), the live test
list (UUID shape of `conversation_id`, the same `generation_id` in prompt and stop, the CLI's interactive gate), real
captures for the fixtures. `p4-antigravity-source`: its `Agent` table (shared code is ready, incl. `StopIfIdle` and
`Base::Profile`; `RealEnv::bases` then fills `profile`) and its entry in `AGENTS` in `hook.rs`.
