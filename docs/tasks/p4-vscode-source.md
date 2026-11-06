# VS Code chat source (Copilot agent mode in VS Code)

Phase: 4. Status: done (new 2026-11-06, Haik: Copilot agent mode in VS Code ships in v1). Needs Haik: no (a live
card follows).
Agent: `wakeshot-worker`. Size: about 130K tokens.
Runs after: `p4-contracts-r2`, `p4-copilot-vscode-research` (done). Parallel with: the other Round 2 source cards
(`p4-opencode-source` Part B, `p4-cursor-source`, `p4-antigravity-source`), docs cards; at most four Rust cards at
once.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-vscode-source\target`; delete it when done.
Real laptop: you may list VS Code's folder names and read your own test copies; never open `state.vscdb` or any file
rule 9 names. No power settings, no screen.

## Goal

`VsCodeSource`, an `AgentSource` like Claude's, Codex's and Copilot's: working, waiting and idle for each chat in VS
Code's chat view (Copilot agent mode and other chat participants), from the chat session files, built from
`docs/research/copilot-vscode-signals.md`.

## Read first

- `CLAUDE.md` (Checks, cargo path, elevation note), `docs/architecture.md` sections 2, 6 and 7.
- `docs/research/copilot-vscode-signals.md`, all of it (rules 1 to 9, "For the source card", the fixture sketch,
  "Open questions only a live test answers").
- `docs/tasks/p4-contracts-r2.md` Result: the read mode for whole files of recently changed files and
  `Liveness::ImageSince`. Its shapes win over the research file's wording.
- One existing source as the house model: `crates/wakeshot-core/src/sources/copilot.rs` and its tests.

## Deliverables

1. `crates/wakeshot-core/src/sources/vscode.rs` (the stub `p4-contracts-r2` made): `VsCodeSource` per rules 1 to 5.
   `wanted()` uses the new read mode on both chat folders under `%APPDATA%\Code\User` (honor `VSCODE_APPDATA` from
   the tray's own environment only, like `CLAUDE_CONFIG_DIR`; Insiders and portable installs: say what you did and
   why). Parser: bytes of one `.jsonl` in, one `SessionFacts` out, replaying `kind` 0 to 3 on `requests`.
2. **Liveness**: rule 7 through `Liveness::ImageSince { images: &["Code.exe"], since_wall_ms: <request timestamp> }`
   unless the contracts Result says otherwise, so a crash leftover stops counting once VS Code restarts. Rule 6's
   `AnyProcess` fallback holds while any `Code.exe` runs and `state.vscdb` always exists, so do not use it alone. Your
   pick, with the reason, is a decision for Haik's review.
3. `label`: "Copilot in VS Code" (the research's name for nearly every user of that view; "VS Code chat" is the other
   option, for Haik's review).
4. Tests in `crates/wakeshot-core/tests/vscode_source.rs` with fixtures in `crates/wakeshot-core/tests/fixtures/
   vscode/`, written from the schema (never copied from a real chat file): working, waiting, idle, working again
   after waiting, a compacted file (one `kind: 0` line), a partial last line (`RetryOnce` then `Unreadable`), an
   unknown `kind` and an unknown `modelState.value` (`UnknownValue`), several requests, `Gone`, an empty window
   session, and that no prompt text reaches any `SessionFacts` field or log text.
5. A "Built" note in `docs/research/copilot-vscode-signals.md` and lines in `docs/architecture.md` section 7 (re-read
   right before each edit; other source cards add lines at the same time).

## Acceptance checks

- `cargo fmt` on your own files; `cargo clippy -p wakeshot-core -p wakeshot-windows --all-targets --locked -- -D
  warnings` and the same without `--all-targets`; `cargo test -p wakeshot-core -p wakeshot-windows --locked`.
  Record each result. A failure in another card's file is reported, never fixed. The orchestrator runs the
  whole-workspace checks.
- Each rule in the research table that you built has a test that fails when the rule is broken (break two on
  purpose and say which).

## Out of scope

- Contracts, module lists and `Cargo` files (stop and ask the orchestrator if you need a change).
- Registering the source in `crates/wakeshot-windows/src/app/tray_mode.rs` or the harness (the live card).
- The Copilot CLI source's `copilot-runtime.exe` and `Code.exe` owners (rule 8: `p4-live-copilot`).

## Rules

- Never run git, not even read-only (`git status`, `git diff`, `git --version`).
- Never read token files, `state.vscdb`, `agent-host.db` or `session.db`. Prompt text is never logged or stored.
- Kill only processes you started, by PID. Never set `RUSTFLAGS`. Cargo may not be on PATH:
  `%USERPROFILE%\.cargo\bin\cargo.exe`.
- Facts only from code or docs you read; mark guesses. Plain short English, no em dashes, no emojis. Write Log lines
  as you go.

## Questions for Haik

None that block. Decisions for review are in the Result.

## Found outside scope

1. **Log name and label differ.** `watch/agents.rs` `source_name` (from `p4-contracts-r2`) logs this source as "VS
   Code chat", while the tooltip label is "Copilot in VS Code". Harmless; align both once Haik picks the label.
2. **`crates/wakeshot-windows/src/app/tray_mode.rs`, doc of `fn sources`**, still says "Claude Code only in v1
   (Codex and Copilot after v1; Haik, 2026-11-03)"; architecture section 7 says Codex and Copilot are in v1 (Haik
   2026-11-06). Stale comment; the live cards that register sources there can fix it.
3. **serde_json's 128-level nesting limit** applies to every JSON source here (Claude, Codex, Copilot, VS Code): a
   line nested deeper does not read. Not seen in any real file; turning it off needs the `unbounded_depth` feature, a
   `Cargo` change.
4. **The research file's rule 1 and "Liveness default: rule 6"** lines are now superseded by its "Built" section
   (Insiders verified, rule 7 used). Left as they were: shared doc, additions only.

## Log

- 2026-11-06: card written by the orchestrator.
- 2026-11-06: worker started; read the research, the contracts Result, `sources.rs`, the Copilot and Claude sources,
  and the watch layer's `Recent` reads.
- 2026-11-06: read VS Code 1.145.1 source as data (`gh api`, scratch folder): `objectMutationLog.ts` (every entry ends
  in `\n`; Delete sets the value to undefined; a Set to undefined has no `v`; Push truncates at `i`, then appends),
  `chatSessionOperationLog.ts` (`modelState` is a primitive, set whole; `requestId` is the key; compaction after 1024
  entries), `chatModel.ts` and `chatServiceImpl.ts` (`reopen()` turns a done request back to Pending with its first
  send time, from a retry on an error card), `userDataPath.ts` (data folder order), `bootstrap-node.ts` (portable
  sets `VSCODE_PORTABLE` inside VS Code only), `test/sanity/src/context.ts` (`Code - Insiders.exe`); VS Code docs
  `profiles.md` (Insiders folder `Code - Insiders`).
- 2026-11-06: `sources/vscode.rs` written (`VsCodeSource`, `installs`, `is_session_file_name`); `cargo check -p
  wakeshot-core` clean. Next: fixtures from the schema and `tests/vscode_source.rs`.
- 2026-11-06: 11 fixtures (script in the scratch folder, from the schema) and 31 tests (3 property tests) written; all
  pass after two test fixes (a done-to-Pending case in a test was a reopen; an id clash in the privacy test). Also
  made a line that is not a JSON object unreadable (serde reads a list into a struct by position). Next: fmt,
  clippy, then break two rules on purpose.
- 2026-11-06: rustfmt on my two files clean; clippy on both packages with and without `--all-targets` clean. Broke
  four rules one at a time (script restores the file, `cmp` checked): rule 4 (NeedsInput read as Pending) failed 5
  tests, rule 7 (oldest open request sets the time) 3, rule 3 (no retry) 4, rule 5 (unknown kind on any path) 2.
- 2026-11-06: `cargo test -p wakeshot-core -p wakeshot-windows --locked` all pass. "Built" note in the research file,
  a VS Code paragraph in architecture section 7, a `docs/research.md` section. Scratch folder (build folder, VS Code
  source copies, scripts) deleted. Status done.

## Result

Status: done, 2026-11-06. Not tested on a real chat and not registered in the tray (the live card does both).

**What was built:**

- `VsCodeSource` (`crates/wakeshot-core/src/sources/vscode.rs`): two `WantedFile::Recent` wants per build,
  `<User>\workspaceStorage` (depth 1, `sub` `chatSessions`) and `<User>\globalStorage\emptyWindowChatSessions`
  (depth 0, its own folder, so `globalStorage`'s constant writes give no notices). Name filter `<plain id>.jsonl`, case
  ignored; `.json` and every other file are never opened.
- `installs(profile, appdata, vscode_appdata)`: Stable (`Code`, `Code.exe`) and Insiders (`Code - Insiders`,
  `Code - Insiders.exe`, both verified in VS Code docs and code), with VS Code's order `VSCODE_APPDATA`, then `APPDATA`,
  then `<profile>\AppData\Roaming`; empty counts as unset. Portable installs are not read: VS Code sets
  `VSCODE_PORTABLE` inside its own process, so the tray cannot see it. The live card calls `installs` from
  `tray_mode.rs` with the tray's own environment.
- Parser: one whole file in, one outcome kept (activity, a time, a problem; no requests, no text). Replays `kind` 0 to
  3 with VS Code's own rules (holes, Push cut at `i`, Delete as undefined), on `requests` only. Working if any
  `modelState.value` is 0, waiting (no time, not remote) if any is 4, else idle. `UnknownValue`: an unknown `kind` on
  a path the source reads, or an unknown value on a chat's newest request. `RetryOnce`, then `Unreadable`: a line that
  does not read, a line that is not a JSON object, or no Initial line. Key: session id from the file name; the same id
  in two folders is one session (the file that says most stands).
- `label`: "Copilot in VS Code". `timers` none, `open_tasks` 0.

**Liveness (decision for Haik's review):** `Liveness::ImageSince { images: ["Code.exe"] (Insiders: its own image),
since_wall_ms }`, where `since_wall_ms` is the newest send time of the requests that decide the activity (all of them
for idle, all after an unknown kind). Why: a crash leftover stays Pending in the file until that chat is opened again;
with rule 7 it stops counting once VS Code starts anew, with no Restart Manager on SQLite (untested) and no reliance on
`state.vscdb`, which always exists, so rule 6's fallback would hold while any `Code.exe` runs. `None` (any start) when
such a request has no usable time (not a number, or before 2020, for example seconds) or went back from done to Pending
or NeedsInput: a retry from an error card keeps the first send time (`chatModel.ts` `reopen()`), and in a later VS Code
run the time test would miss it. Change it when the live test shows a window closed or reloaded mid-request leaves a
Pending (research open question 4: it would hold until VS Code exits), then add the `code.lock` pid; or when a
compacted reopened request is seen (its history is gone, so it falls back to its first time).

**Other choices** (mine), each with when to change it:

1. Label "Copilot in VS Code" (card default; "VS Code chat" is the other option). For Haik's review.
2. Unknown values are gated: a new `kind` counts only on a path the source reads, an unknown state only on the newest
   request. Reason: here `UnknownValue` holds while VS Code runs, not while one session's process runs, so an ungated
   new kind of write (say, appending answer text) would hold the PC all day for every chat of the last 24 hours. Change
   it when a live format change shows a working state the gate hides.
3. A partial last line follows the contract: `RetryOnce`, then `Unreadable` (no hold, notice). Change it when live
   logs show the notice from files cut by a crash: then use the complete lines on the retry.
4. Idle chats are reported too, like the other sources (cheap: liveness checks are deduped). Change it when the live
   test shows many recent chats slow a round, or exit waits churn on short-lived `Code.exe` children: then report only
   chats that are not idle.
5. A request index of 65536 or more is skipped. Change it when a real chat has that many requests.

**Files changed:** `crates/wakeshot-core/src/sources/vscode.rs` (was a stub); `crates/wakeshot-core/tests/
vscode_source.rs` (new); `crates/wakeshot-core/tests/fixtures/vscode/` (new: 11 `.jsonl` from the schema and a
`README.md`); `docs/research/copilot-vscode-signals.md` ("Built" section added); `docs/architecture.md` (section 7, a
VS Code paragraph added); `docs/research.md` ("VS Code chat source checks" added); this card.

**Tests:** 31 in `tests/vscode_source.rs`, 3 of them property tests (a fixture cut at any byte reads at a line end or
asks for a retry; any bytes; random lines near every edge). Covered: working, waiting, working again, idle, compacted
(also rewritten in place, shorter), partial last line (`RetryOnce`, then `Unreadable`, and back), unknown `kind` and
unknown `modelState.value` (`UnknownValue`, with the gates), several requests (a Push with `i`), reopened, send times,
`Gone`, empty window, one id in two folders, Insiders, VS Code's write rules, files outside the chat folders, the name
filter, `installs`, and no prompt text in any `SessionFacts` field, `Debug` or format text. Broken on purpose (Log):
rule 4 (5 tests failed), rule 7 (3), rule 3 (4), rule 5's gate (2); file restored and checked with `cmp`.

**Checks** (own build folder, `--locked`, no `RUSTFLAGS`):

- `rustfmt --edition 2024 --check` on my two Rust files: clean (not `cargo fmt`: another card edits `opencode.rs` in
  the same package).
- `cargo clippy -p wakeshot-core -p wakeshot-windows --all-targets --locked -- -D warnings`: clean.
- `cargo clippy -p wakeshot-core -p wakeshot-windows --locked -- -D warnings`: clean.
- `cargo test -p wakeshot-core -p wakeshot-windows --locked`: all pass. Core unit 707 (1 ignored), `claude_status` 11,
  `claude_transcript` 25, `codex_source` 18, `copilot_source` 42, `opencode_source` 14 (another card's), `vscode_source`
  31; `wakeshot-windows` 649 (24 ignored).
- No contract, module list or `Cargo` change.

**Open items:** the live card registers the source in `tray_mode.rs` and the harness and answers the research's open
questions 1 to 5 (write delay, approval writes, window close or reload mid-request, crash then restart); Found outside
scope 1 to 4.
