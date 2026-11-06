# Phase 4: Sources (track)

Status: planned 2026-11-03 by card `p4-plan-sources`; Haik signed off all five track plans 2026-11-03 18:09, every
question with its default. Cards: see the table.

## Goal

Everything that tells Wakeshot something needs the PC: Claude Code (status files and transcripts, with timers), Codex
CLI and Copilot CLI, hold files (`wakeshot run`, Keep awake, any script), and the `wakeshot` command itself. The
parsers live in the core (bytes in, facts out); the watch thread in `wakeshot-windows` feeds them, checks liveness and
posts `Event::Sources`, `Event::Holds`, `ConfigChanged` and `Menu(Restore)`. Built test-first against the contracts in
`crates/`, beside tracks 2, 3, 5 and 6.

The design is `architecture.md` sections 4 (CLI, watch thread), 6 (hold files, config) and 7 (sources), and
`research/claude-code-signals.md` "Recommended detection rules". The notes below cover only what those leave open.

## Track notes

Values this plan sets, each with why. Haik signed them off with the plan (2026-11-03 18:09).

1. **No cron or date crate.** Claude's `CronCreate` takes only `*`, numbers, steps, ranges and lists, in local time
   (docs, research "Sources track plan checks"). A small parser and next-time function in `sources::cron` covers it;
   `croner` needs `chrono` or `jiff`, and `jiff`'s defaults pull system time zone code and a time zone database into
   the core. RFC 3339 parse and format for hold `expires` is also our own (`holds::file`). Change it when a source
   needs time zones other than the PC's own, or the own code passes about 400 lines.
2. **Local time comes in from outside.** Cron times are local. The parser gets a `LocalOffset` (UTC ms in, offset
   out) when it is built; `wakeshot_windows::localtime` gives the PC's zone with its daylight saving rules, and tests
   give fixed or made-up zones. So the core stays free of clock reads and OS calls.
3. **Claude sessions are keyed by pid.** `sessionId` changes at `/clear` (research, live test 9), so `SessionKey.id`
   is the pid as text, and timers carry across `/clear` (rule 5).
4. **A timer's due time includes Claude's jitter.** A recurring job runs late by up to 30 minutes; a due time without
   it would end the hold before the run (the core's margin is 2 minutes). `nextFireAt` already includes it; later
   runs are computed with the documented formula.
5. **A cron that does not parse** (should not happen: Claude rejects other syntax) holds only for the exact
   `nextFireAt` it was created with; later runs do not hold, and the source reports `UnknownValue("cron", ..)` so
   "update Wakeshot" shows once.
6. **Transcript lines of a known kind with a missing field** are skipped and reported as `UnknownValue` with the line
   kind; the status file signal for that session stays. Unknown line kinds are ignored silently (transcripts have
   many).
Notes 7, 8 and 9 apply: Codex and Copilot are back in v1 (Haik, 2026-11-06; on 2026-11-03 they had moved to after v1).

7. **Codex and Copilot: the lock file ties a session to its process.** The watch thread asks Restart Manager which
   process holds the lock file open and checks that pid and start time. The Codex live test (2026-11-06, 0.167.0)
   showed Restart Manager names the session's own `codex.exe`, so the weaker rule ("any `codex.exe` alive and the
   lock file exists", "can outlive its session") is only the fallback when Restart Manager fails.
8. **Codex goals (`goals_1.sqlite`) are not read in v1.** Between two goal turns the gap is seconds, far below any
   sleep timeout; reading SQLite would add a dependency for nothing.
9. **Copilot schedules are computed in the PC's own zone.** Their events carry a `tz`; Copilot runs on the same PC,
   so it should be the PC's zone (not verified; `p4-live-copilot` checks what `tz` holds). Mapping IANA names to
   Windows zones is not worth it for v1.
10. **Hold files.** A file whose `expires` does not parse is ignored (no hold) and logged, like a file that is not
    JSON; the user doc says so. A file is re-read once 200 ms later before it counts as bad, since scripts may not
    write atomically. Temp files of Wakeshot's own atomic writes must not end in `.json` (a need on the power track).
11. **`wakeshot run` finds the program like PowerShell does:** `PATH` plus `PATHEXT`, not the current folder unless
    the command names a path. `.exe` and `.com` start directly with the arguments exactly as typed; `.bat` and
    `.cmd` go through `cmd.exe /d /s /c`; anything else (such as `.ps1`) gets an error that names `pwsh -File`. Rust's
    `Command` cannot find the program itself (it ignores `PATHEXT`, research), so Wakeshot resolves the full path
    first and passes the arguments with `raw_arg`.
12. **`wakeshot run` and Ctrl+C:** the wrapper installs a handler that returns TRUE for Ctrl+C and Ctrl+Break. The
    NULL form would be inherited by the child, which then could not be stopped (research).
13. **A global `--dry-run` flag on the CLI** uses the dry-run data folder (`%LocalAppData%\Wakeshot\dry-run`), the
    same as `wakeshot-tray --dry-run`, so tests and live checks never touch the real data folder. Listed in the help
    as a test aid. Unit tests use a `DataDir` on a temp folder (or `WAKESHOT_DATA_DIR`, which the power track's
    `store` reads in debug builds).
14. **The watch thread posts a snapshot only when it differs from the last one it posted** (plus once after each
    start), so status file bursts do not fill the loop's message queue (10,000 message limit).
15. **`restore-request.json`** gives one `Menu(Restore)` per appearance of the file, never one per change notice;
    deleting it is the IO thread's job after `state.json` is written (power and core tracks).
16. **Seams from the contracts card.** The watch thread posts through the `EventSink` the contracts card adds (review
    M1), logs through the power track's `log::Log` (fake `MemoryLog`), reads data folder paths from `store::DataDir`,
    and checks processes through the power track's `process::Liveness` trait (fake `FakeLiveness`) plus its
    `ProcessHandle` and `ExitWait`. Two additions this track needs on `process::Liveness` (`start_of_pid`, `list`) are
    in the plan card's Result (`p3-process` builds them, review M3). Tests use the fakes, so no watch card waits for
    the power track, except `p4-watch-thread`, which needs the real `ProcessHandle` and `ExitWait` (`p3-process`;
    they have no fake; review M4). The CLI cards call the real `store`, `process` and `log`, so they wait for
    `p3-store`, `p3-process`, `p3-log`.
17. **No rival code is ported.** The Codex and Copilot rules come from our own reading of their sources. If a card
    ports logic from an MIT rival after all, it records the credit line for the README (decision 9).
18. **Checks while other cards run** (cross-track review B1; the rule is `docs/plan.md` "How we work" step 2). Every
    card of this track checks only its own package and files:
    - format: `rustfmt --edition 2024 --check <owned files>`. A bare `rustfmt` uses the 2015 edition (the repo has no
      `rustfmt.toml`) and also checks the child modules a file declares (both checked 2026-11-03, `docs/research.md`
      "Track plan fixes checks"), so a diff in another card's child file is not this card's. Format only owned files
      (`rustfmt --edition 2024 <file>`), never a file whose child modules another card owns.
    - lint: `cargo clippy --locked -p <own package> --all-targets --target <t> -- -D warnings` and the same without
      `--all-targets` (the libs as they ship, without `test-support`; `CLAUDE.md` "Checks"), for both targets: nothing
      in the owned files.
    - test: `cargo test --locked -p <own package> <filter>`: the card's own tests pass. Several filters go after `--`
      (`cargo test --locked -p wakeshot-windows -- a:: b::`); two filters before `--` are a cargo error.
    - Every cargo command takes `--locked`: it fails at once if a build would change `Cargo.lock`, which only
      `p6-velopack-dependency` changes, alone.
    - A failure in a file another card owns goes in the Log: wait and run again; never edit, format or `--fix` that
      file. Never run `cargo fmt` without `--check`, `cargo fix` or `cargo clippy --fix`.
    - Every save of an owned file compiles (`todo!()` bodies are fine): one compile error fails every `-p` run of that
      crate and of the crates that use it.
    - No card runs `--workspace` or `--all` as a done check; the orchestrator runs the whole-workspace check when it
      closes a wave.
    - Once `p6-velopack-dependency` is done, ARM64 builds of `wakeshot-tray` need clang (only the tray depends on
      `velopack`). Haik said yes to the Build Tools clang component (sign-off item 8, Haik 2026-11-03 18:09);
      `p6-velopack-dependency` step 0 has Haik install it, elevated. Without it the tray's ARM64 builds are CI only
      (review M5).
    - Delete the card's own target folder when the card is done (review m14).

## Cards

| Card | Needs Haik | Runs after | Status |
|---|---|---|---|
| `p4-hold-files` | no | contracts card | done (20:03): 21 tests, `docs/hold-files.md` |
| `p4-cron` | no | contracts card | done (21:53): 34 tests; matches Claude's own scheduler in 400,000 cases |
| `p4-claude-status` | no | contracts card | done (21:37): 34 tests |
| `p4-claude-live-shapes` | no; runs Claude Code test sessions (Haiku); agent-session lane | contracts card | done (18:47): 27 fixtures; resume and rule 3, 5 changes in the signals doc |
| `p4-codex-source` | no | contracts card | done 2026-11-06 (Codex 0.167.0 read, not tested live; reverted threads need a contract change) |
| `p4-watch-folders` | no | contracts card | done (00:26): 20 tests, no handle leak in 1,000 cycles |
| `p4-watch-reads` | no | contracts card | done (21:12): 18 tests; `names` returns `io::Result` |
| `p4-cli-args` | no | contracts card | done (committed 01:14): every command routed; the child text comes from the raw command line |
| `p4-claude-transcript` | no | `p4-claude-status`, `p4-cron`, `p4-claude-live-shapes` | done (22:35): 46 tests; no contract change |
| `p4-copilot-source` | no | `p4-cron` | done 2026-11-06 (schema of 1.0.106, not tested live; 42 tests) |
| `p4-copilot-vscode-research` | no | none | done 2026-11-06: a partly reliable signal, medium confidence (`research/copilot-vscode-signals.md`); a source card follows, after a contracts change run alone |
| `p4-opencode-source` | no | Part B after `p4-contracts-r2` | Part A done 2026-11-06 (`research/opencode-signals.md`); Part B done (2026-11-06 ~17:55, 1c58f43): 2.x only, 14 + 8 tests, the table test (caught a planted `JOIN message`), a log leak test; a VFS shim so a read never creates `-wal` or `-shm`; minimum SQLite 3.22.0; 1.x question for Haik (default no); review running (`docs/reviews/p4-opencode-db-read.md`) |
| `p4-vscode-source` | no | `p4-contracts-r2` | done (2026-11-06 ~17:45, 5869c5c): 31 tests; `ImageSince` on `Code.exe` with the newest send time; label "Copilot in VS Code"; Insiders read, portable not (worker calls, for Haik's review); not live yet |
| `p4-contracts-r3` | no | `p4-contracts-r2`, the hooks security review and its fixes; runs alone among Rust cards | done (2026-11-06 ~19:30, 98c8795): the six hooks contract items, the OpenCode `-shm` loop fix, Copilot, VS Code and OpenCode in the tray; 1622 tests; whole-workspace checks green on x64 and ARM64; one `windows` feature added (`Wdk_System_Threading`) |
| `p4-cursor-source` | no: Cursor 3.27.12 passes hook input through a temp file on every plugin path, so the plugin ships | `p4-contracts-r3`, the hooks review verify | done (2026-11-06 ~20:50, f9a41e2): 21 + 8 core, 17 + 4 CLI tests; privacy and exit code rules broken on purpose and caught; `hook <agent> remove` wired |
| `p4-agent-hooks-install` | no | `p4-contracts-r3`, the hooks review verify | done (2026-11-06 ~20:30, f46828f): 23 tests; link check and fixed file set broken on purpose and caught; the tray call and the CLI remove call not wired yet (to `p4-antigravity-source` and `p4-cursor-source`) |
| `p4-antigravity-source` | only if its payload is on the command line | `p4-cursor-source`, `p4-agent-hooks-install` | running (2026-11-06 ~20:50), with the Cursor and Antigravity tray wiring |
| `p4-live-vscode` | no | `p4-vscode-source`, `p4-live-harness`; live cards in parallel since 18:50 | running (2026-11-06 ~19:35) |
| `p4-live-opencode` | no | `p4-opencode-source`, the OpenCode review fixes; live cards in parallel since 18:50 | running (2026-11-06 ~20:30) |
| `p4-live-cursor` | only if the payload check finds the command line | `p4-cursor-source`, `p4-agent-hooks-install`, the hooks code review fixes; live lane after `p4-live-opencode` | card written 2026-11-06 |
| `p4-live-antigravity` | only if its payload is on the command line | `p4-antigravity-source`, `p4-live-cursor` | card written 2026-11-06 |
| `p4-core-linux-tests` | no | none | done (2026-11-06 ~20:10, 750b023): Codex and OpenCode tests join one level at a time; CI core step runs `--no-fail-fast`; three VS Code tests sent to `p4-live-vscode`; no Linux run here (rustup in WSL refused by the permission check; no C compiler there) |
| `p4-cursor-gemini-research` | no | none | done 2026-11-06 (`research/cursor-gemini-signals.md`): Cursor big, Gemini CLI enterprise only since 2026-06-18; hooks question for Haik |
| `p4-watch-data` | no | `p4-hold-files`, `p4-watch-reads` | done (21:12): 19 tests |
| `p4-watch-agents` | no | `p4-watch-reads` | done (22:43): 19 tests; API changes for `p4-watch-thread` in its Result |
| `p4-watch-thread` | no | `p4-watch-folders`, `p4-watch-data`, `p4-watch-agents`, `p3-process` | done (committed 00:54): 14 thread tests, 2 m3 tests; each of 4 behaviours caught by its tests when broken |
| `p4-cli-run` | no | `p4-cli-args`, `p4-hold-files`, `p3-store`, `p3-process`, `p3-log` | done (committed 01:14): `child.rs`, `run.rs`, core `holds::file::render`; 75 CLI tests, 3 exe, 5 run, 14 child, 24 holds file |
| `p4-cli-commands` | no; screen once (opens one file in the default app; global lane, review M6) | `p4-hold-files`, `p3-store`, `p3-process`, `p3-log` | done (00:36): 30 tests; screen step ran once |
| `p4-cli-restore` | no (Phase 1 Question 12: the default, Haik 2026-11-03 18:09; nothing to add) | `p3-journal`, `p3-power-plan`, `p2-ownership-restore`, `p3-store`, `p3-process`, `p3-log` | done (00:36): 10 tests |
| `p4-live-claude` | optional: 3 minutes for the VS Code check; agent-session lane | `p4-watch-thread`, `p4-claude-transcript`, `p4-cli-run`, `p3-process`, the Claude parsing review (`docs/phases/02-06-lighter.md`) | done (committed 01:59): every live case passed (Claude Code 2.1.317); m6 fixed; the VS Code check waits for Haik |
| `p4-live-codex` | no: Codex logged in by Haik (2026-11-06); agents run it with `p4-live-harness` | `p4-contracts-r2`, `p4-live-harness`; one live card at a time | done (2026-11-06 ~18:25, a1ca2ce): Codex 0.167.0, 7 one-line prompts; holds 0.3 s after a turn starts, ends in 17 ms; reverted thread followed; crash ends the hold in 115 ms; Codex in the tray |
| `p4-live-copilot` | no (Round 3) | `p4-contracts-r2`, `p4-live-harness`; live cards in parallel since 18:50 | done (2026-11-06 ~21:00, 00e0120): Copilot 1.0.106, 7 prompts; posts 0.13 to 0.25 s after each line; liveness fixed (the `.hold` holder named in the file); crash ends the hold in 61 ms; schedules live |
| `p4-watch-tick-flake` | no | `p4-watch-thread` | done (15:00: ticks wait for their check; data files written atomically; 20 serial and 24 of 24 load runs pass) |
| `p4-contracts-r2` | no | the four 2026-11-06 source and research cards (done); runs alone among Rust cards | done (2026-11-06 ~16:20, 24f7405): `FindNewest` (Codex reverted threads) and `Recent` as new variants, `Query`, `ImageSince`; 1541 tests; whole-workspace checks green on x64 and ARM64; no Cargo change |
| `p4-antigravity-research` | no | none | done (2026-11-06): CLI 1.5.2 (`agy.exe`) and IDE 2.6.0 both need hooks (a Wakeshot plugin folder under `~\.gemini\config\plugins\`); free test yes (Google's claim; Armenia listed); build in v1 (`research/antigravity-signals.md`) |
| `p4-hooks-design` | no; a security review reads it before any build | `p4-cursor-gemini-research` | done (2026-11-06): `docs/agent-hooks.md`; one command `wakeshot.exe hook <agent> <event>` from a Wakeshot plugin folder per agent, state files read with `Folder` and `Process`; `main.log` dropped; security review running (`docs/reviews/p4-hooks-design-security.md`) |
| `p4-live-harness` | yes: one sign-in sitting (about 10 min, Cursor and Antigravity only; steps in the card) | `p4-live-claude`; Part B after `p4-contracts-r2` | Part A done (2026-11-06 ~17:05, bb53377): every agent installed at Medium, Codex, Copilot CLI and VS Code logins reused, OpenCode on Ollama; ConPTY driver `spikes/agent-drive/` works; Part B done (d119019): `agent-watch --source claude,codex,copilot` |

Bundles (one worker, both cards in order; `docs/phases/02-06-lighter.md`): `p4-cli-args` + `p4-cli-run`;
`p4-cli-commands` + `p4-cli-restore`; `p4-watch-reads` + `p4-watch-data`. Codex and Copilot moved to after v1 (Haik,
2026-11-03): their four cards are `later`, and the lock-holder code moved from `p4-watch-reads` and
`p4-watch-agents` into `p4-codex-source`.

"Contracts card" is the one card the orchestrator runs alone before the builds (this track's changes are in the
Result of `docs/tasks/p4-plan-sources.md`). `p3-*` cards are the power track's (`docs/phases/03-windows-power.md`);
`p3-store`, `p3-process` and `p3-log` are in its first wave.

Cards that use the real laptop (review M6, m19):

- Agent-session lane: `p4-claude-live-shapes`, `p4-live-claude`, `p4-live-copilot` and `p4-live-codex`, start agent
  test sessions (no power settings, screen or sound). They run one at a time with each other, and may run beside the
  global lane (power, screen, sound, installs), so they never block screen or power cards for hours. `p4-live-codex`
  and `p4-live-copilot` also need Haik's hands and may install an agent CLI (an install: never beside another install
  or `p6-install-e2e`).
- Global lane: `p4-cli-commands`, once, for the one `log` check that opens a temp file in the default app (a window
  on Haik's screen). Every other test of that card is a plain unit test.

No card of this track writes power settings or plays sound.

### Files each card owns

No two cards own a file at the same time; a later card takes a file over only after the earlier one is done. Every
file below exists after the contracts card (the split asked by `docs/reviews/p1-contracts-domain.md`, B1), so no card
of this track edits a module list, a contract file, a `Cargo.toml` or the CLI's `main.rs` and `command.rs` (the
contracts card writes the dispatch, then they stay as they are).

| Card | Owns |
|---|---|
| `p4-hold-files` | `crates/wakeshot-core/src/holds/file.rs`; `docs/hold-files.md` |
| `p4-cron` | `crates/wakeshot-core/src/sources/cron.rs`; `crates/wakeshot-windows/src/localtime.rs` |
| `p4-claude-status` | `crates/wakeshot-core/src/sources/claude/mod.rs`, `.../claude/status.rs`; `crates/wakeshot-core/tests/claude_status.rs`; `crates/wakeshot-core/tests/fixtures/claude/status/` |
| `p4-claude-live-shapes` | `crates/wakeshot-core/tests/fixtures/claude/live/`; a new section in `docs/research/claude-code-signals.md` |
| `p4-claude-transcript` | `.../claude/transcript.rs`; `.../claude/mod.rs` and `crates/wakeshot-core/tests/claude_status.rs` (both after `p4-claude-status`: the `new(home, zone)` change breaks that test file, review m4); `crates/wakeshot-core/tests/claude_transcript.rs`; `crates/wakeshot-core/tests/fixtures/claude/transcript/` |
| `p4-codex-source` | `crates/wakeshot-core/src/sources/codex.rs`; `crates/wakeshot-core/tests/codex_source.rs`; `crates/wakeshot-core/tests/fixtures/codex/`; `crates/wakeshot-windows/src/lockowner.rs`, and the `AnyProcess` part of `watch/agents.rs` (after `p4-watch-agents`) |
| `p4-copilot-source` | `crates/wakeshot-core/src/sources/copilot.rs`; `crates/wakeshot-core/tests/copilot_source.rs`; `crates/wakeshot-core/tests/fixtures/copilot/` |
| `p4-watch-folders` | `crates/wakeshot-windows/src/watch/dir.rs` |
| `p4-watch-reads` | `crates/wakeshot-windows/src/watch/read.rs` |
| `p4-watch-data` | `crates/wakeshot-windows/src/watch/data.rs` |
| `p4-watch-agents` | `crates/wakeshot-windows/src/watch/agents.rs` |
| `p4-watch-thread` | `crates/wakeshot-windows/src/watch/thread.rs` |
| `p4-cli-args` | `crates/wakeshot-cli/src/args.rs` |
| `p4-cli-run` | `crates/wakeshot-windows/src/child.rs`; `crates/wakeshot-cli/src/run.rs` |
| `p4-cli-commands` | `crates/wakeshot-cli/src/keep_awake.rs`, `automatic.rs`, `status.rs`, `open_log.rs`; `crates/wakeshot-windows/src/shell.rs` |
| `p4-cli-restore` | `crates/wakeshot-cli/src/restore.rs` |
| `p4-live-claude` | `crates/wakeshot-windows/examples/agent-watch.rs`; a new section in `docs/research/claude-code-signals.md` (after `p4-claude-live-shapes`); fixes in `sources/claude/*`, `sources/cron.rs`, `holds/file.rs`, `watch/*` and their tests and fixtures (all done by then) |
| `p4-live-codex` | `crates/wakeshot-core/tests/fixtures/codex/live/`; a new section in `docs/research/other-agent-signals.md`; fixes in `sources/codex.rs`, `tests/codex_source.rs`, `lockowner.rs`, `watch/agents.rs` (all done by then) |
| `p4-live-copilot` | `crates/wakeshot-core/tests/fixtures/copilot/live/`; a new section in `docs/research/other-agent-signals.md` (after `p4-live-codex`); fixes in `sources/copilot.rs`, `tests/copilot_source.rs`, `lockowner.rs`, `watch/agents.rs` (all done by then) |

Every card may append to `docs/research.md` (additions only, re-read right before each edit).

### Order inside the track

1. Right after the contracts card, in parallel: `p4-hold-files`, `p4-cron`, `p4-claude-status`, `p4-watch-folders`,
   the `p4-cli-args` + `p4-cli-run` bundle (it starts after `p4-hold-files`), and `p4-claude-live-shapes`
   (done). `p4-hold-files` first if workers are short: the core track's Keep awake card needs its `format_expires`.
2. Then: the `p4-watch-reads` + `p4-watch-data` bundle (after `p4-hold-files`), `p4-claude-transcript`,
   `p4-watch-agents` (after `p4-watch-reads`).
3. Then: `p4-watch-thread` (also after `p3-process`); the `p4-cli-commands` + `p4-cli-restore` bundle (after
   `p3-journal`, `p3-power-plan` and the core's `p2-ownership-restore`; review M1; the tray track's `p5-haik-check`
   waits for its `shell.rs`).
4. Then: the Claude parsing review (`docs/phases/02-06-lighter.md`), then `p4-live-claude`.
5. Back in v1 (Haik, 2026-11-06): `p4-codex-source`, `p4-copilot-source` (done), then `p4-live-copilot`, `p4-live-codex`,
   one at a time.

Integration (planned after the tracks, not here): start the watch thread in the tray with the Claude source and the
real `process::Liveness`; post its events to the loop window after `Started`; delete hold files on `DeleteHold`; the
end-to-end checks of `wakeshot run`, `keep-awake`, `status` and `restore` against a running tray.

Size: about 1.3M tokens of worker context for v1 by the card estimates (50K to 140K each), which ran 2 to 3 times low
in wave 1; the four later cards and the lock-holder code are about 400K more.

## Questions for Haik

Answered 2026-11-03 18:09, both with the default (`docs/phases/02-06-signoff.md` items 1 and 2): Codex holds with no
limit; Copilot waiting does not hold. The cards build these.

1. **Codex waiting for approval looks like working.** Codex does not write its approval prompts to disk, so a Codex
   turn that waits for you looks the same as one that works, and it holds with no time limit while `codex.exe` lives
   (also a turn that sleeps up to 12 hours inside its `sleep` tool). Default: hold, no limit (decision 12: the
   process and its open turn are the fact; the battery floor still protects). The other option: a 1 hour limit on a
   Codex turn with no new rollout line, which would also end long silent builds.
2. **Copilot waiting for approval.** Copilot writes `permission.requested`, so Wakeshot can tell. Default: no hold,
   the same as Claude waiting without Remote Control. The other option: hold up to 1 hour like Claude with Remote
   Control.

## Work log

- 2026-11-03 16:51: phase file and cards written by `p4-plan-sources`.
- 2026-11-03: Haik signed off with every default (18:09); `p1-close-contracts` wrote the answers into this file and
  the cards.
- 2026-11-03 20:12: `p4-hold-files` done and committed. Open, for whoever next edits these files: two stale lines in
  `architecture.md` section 6 ("before" should be "at or before"; the old `jiff` option) and an old pointer in the
  `files.rs` module doc (from its Result).
- 2026-11-03 21:14: the `p4-watch-reads` + `p4-watch-data` bundle done and committed; `p4-claude-status` running.
  Open, for the docs pass: `architecture.md` section 8 says this crate returns `WinError`, but `watch/read.rs` returns
  `io::Result` as its card says. For `p4-watch-agents`: on `restarted`, feed `Gone` and then `Appended`.
- 2026-11-03 21:55: `p4-cron` done and committed; `p4-claude-transcript` running. For the Claude parsing review:
  the jitter constants come from a remote flag (`tengu_kairos_cron_config`). For a docs pass:
  `docs/research/claude-code-signals.md` says "15 s early" for `*/5`; `p4-cron` found 285 s after the last fire.
- 2026-11-03 22:36: `p4-claude-transcript` done and committed. A transcript problem reports `UnknownValue` only while
  the session already holds, else `Ok` (so an idle session never shows "update Wakeshot"; a notice-only variant is
  proposed in its card). For the Claude parsing review: an SDK session's task stopped by a Monitor timeout or the UI
  writes no transcript line, so it holds until the session ends; transcript times use `holds::file::parse_expires`.
- 2026-11-03 23:36: Claude parsing review running (`wakeshot-reviewer`, report `docs/reviews/p4-claude-parsing.md`), scope `p4-claude-status`, `p4-cron`, `p4-claude-transcript`, `p4-hold-files`, `p4-watch-reads`, `p4-watch-agents`.
- 2026-11-03 23:56: Claude parsing review done (`docs/reviews/p4-claude-parsing.md`): 1 blocker, 1 major, 8 minors; no `.key` open, no prompt text stored or logged. B1: a task stopped with `TaskStop` (or `KillShell`, `KillBash`) never closes, so VS Code and SDK sessions hold until the chat closes. M1 (known item 1, confirmed): a Monitor that times out holds until the session ends. Routing: B1, M1, m1, m2, m3, m4, m5, m7 and m8's third bullet to a Claude fix worker at the next free build slot (it reads `p4-claude-transcript`'s Log); m8's first two bullets to the running docs pass; m6 to `p4-live-claude` (its Log). Then the same review brief checks B1 and M1 only; that check gates `p4-live-claude`.
- 2026-11-04 00:02: Claude fix worker running (`p4-claude-fixes`, Log in `p4-claude-transcript`): B1, M1, m1 to m5, m7, m8 rule 3.
- 2026-11-04 00:06: `p4-watch-folders` and the `p4-cli-commands` + `p4-cli-restore` bundle running (the bundle holds the screen lane for its one file open).
- 2026-11-04 00:26: `p4-watch-folders` done and committed; `p4-watch-thread` starts. Decided by the worker, for Haik's review: watches stay open all the time, so renaming a watched folder's parent (for example `~/.claude`) fails with Access denied while the tray runs; deleting still works. For the next docs pass: `architecture.md` section 5 "Folder changes" row still says several folders on one thread are not tested; this card tested it.
- 2026-11-04 00:30: Claude fixes done and committed (Log in `p4-claude-transcript`): B1, M1, m1, m2, m4, m5, m7, rules 3 and 5, product.md lines. Decided by the worker, for Haik's review: a computed recurring cron time is reported twice (built-in and the latest the flag allows); the 7-day end stays; a UI stop holds until its `killed` notice lands or the session ends (the output file's `[killed]` trailer is not read); a Monitor of unknown shape ends after 1 hour. m3 moved to `watch/agents.rs` and went to `p4-watch-thread`. The verify of B1 and M1 runs (`docs/reviews/p4-claude-parsing-verify.md`). Open: the README should mention the UI-stop case (`p6-readme`).
- 2026-11-04 00:36: the `p4-cli-commands` + `p4-cli-restore` bundle done and committed (40 CLI tests). Decided by the worker, for Haik's review: `keep-awake off` works with or without the tray; `status` exits 0 for "not running"; `wakeshot log` with no log exits 0; the CLI sets a corrupt journal aside, and a newer journal restores the keys this version knows and exits 1; the CLI does not start the tray after its own failed restore. For `p6-int-io-thread`: `shell::open_with_default_app(target: &OsStr)` is the opener (own COM thread, at most 10 s); `log.rs` has no public log path yet. The `p4-cli-args` + `p4-cli-run` bundle started: orchestrator decision, no `serde_json` in the CLI (a Cargo change runs alone); the bundle adds a public hold file writer to the core's `holds/file.rs`, which owns the format.
- 2026-11-04 00:52: orchestrator handover (new session). The Claude fixes verify is committed
  (`docs/reviews/p4-claude-parsing-verify.md`): B1 fixed; M1 part a fixed; M1 part b (a UI stop's killed notice sits
  in a `queue-operation` line that `transcript.rs` skips; from the 2.1.317 code only) stays a major; 3 minors. CI on the
  pushed 36416ae failed: 6 `watch/data.rs` tests use pid 4, which the Claude fix round (7ec7eda) now rejects as the
  System process. One worker runs both fixes (the `data.rs` tests, then M1 part b); the same verify brief checks M1
  part b after it. `p4-live-claude` waits for that verify.
- 2026-11-04 00:56: `p4-watch-thread` done and committed, with the Claude review's m3 (a `sessions` listing error
  now reads the known status files). `cargo test -p wakeshot-windows`: 367 passed, 6 failed (the `watch::data` pid 4
  tests, fix running). Decided by the worker, for Haik's review: a restore request whose post failed is posted again at
  the next tick if the request file still exists; an agent folder made after start is first watched at the next tick
  (up to 30 s late), `holds\` at once. The `p4-watch-folders` rename question stays open for Haik. For
  `p6-int-tray-mode`: `WatchThread::start(dir, config, sources, liveness, sink, log)` takes `Started`'s config; start
  it after `Started` is posted; call `tick()` from `watch_tick`. For `p4-live-claude`: call `tick()` every 30 s and
  pass `Config::default()`.
- 2026-11-04 01:08: the fix worker is done and committed: the six `watch::data` tests now use pid 40
  (`cargo test -p wakeshot-windows --lib`: 387 passed, 0 failed); M1 part b fixed in `transcript.rs`: an `enqueue`
  line closes the tasks its notice names (all 10 delivered notices in the live fixtures have that line first, 0 to
  3.4 s earlier; not checked live, `p4-live-claude` checks it). Verify minors N1 and N3 answered; N2 goes to
  `p4-live-claude`. Decided by the worker, for Haik's review: a UI stop now stops holding as soon as its queued notice
  is on disk (this replaces the earlier orchestrator decision "a UI stop holds until its killed notice"); it still
  holds longer if the remote flag `tengu_rosy_donut` is off or a hook holds the enqueue back (both not verified); only
  `enqueue` lines are read, not `remove`. For the next docs pass: `docs/product.md:41-46` still says a UI stop holds
  because the output file is "the only other trace" (wrong now; proposed text in the card); `docs/architecture.md`
  899-900, the `timers` and `open_tasks` comments, do not match the Monitor rules. Harmless: `watch/read.rs:577`
  writes pid 4 bytes it never parses. The second verify (M1 part b) runs: `docs/reviews/p4-claude-parsing-verify2.md`.
- 2026-11-04 01:14: the `p4-cli-args` + `p4-cli-run` bundle done and committed. `wakeshot --dry-run run -- cmd /c
  echo hi` from the built exe printed hi and exited 0; the five open `--dry-run` exe checks of `p4-cli-commands` and
  `p4-cli-restore` matched the in-process results. Decided by the worker, for Haik's review: wrapper exit codes 127
  (not found), 126 (cannot start), 125 (child lost); a batch file runs through its resolved full path, so cmd never
  searches the current folder; ASCII case-insensitive words; `-h` and `/?` also show help; the warning words. For
  `p4-live-claude`: untested are a real Ctrl+C in Windows Terminal and conhost, Ctrl+Break, closing the window while run
  waits, and run against a live tray. For `p6-int-dry-run-e2e`: `tests/run.rs` drives the exe on a temp folder with
  `WAKESHOT_DATA_DIR` (debug builds only). Outside scope: `keep_awake.rs` writes its hold file by hand and could use
  `render`; architecture section 4 does not name the exit codes (next docs pass). Session `wakeshot-91` has no agent
  left; this orchestrator runs every card from here.
- 2026-11-04 01:20: the second Claude verify is committed (`docs/reviews/p4-claude-parsing-verify2.md`): M1 part b
  Fixed in code for background shells and shell Monitors (the panel stop itself not yet run live); 0 blockers, 0 majors,
  3 minors (an MCP or WebSocket Monitor's UI stop writes no `<status>`, so it holds until its timeout, unverified; the
  stale `product.md` and `architecture.md` lines already tracked; rule 3 cites `product.md:44-50` for "No time cap",
  which is at 38-40). The Claude parsing review has passed. Pushed main at 01:19 (16 commits, push scan clean), which
  carries the `watch::data` test fix. `p4-live-claude` running (agent-session lane).
- 2026-11-04 01:59: `p4-live-claude` done and committed. Live run 01:27 to 01:53, Claude Code 2.1.317, Haiku: cases 1
  to 12 and 14, resume (`procStart` divider), `/clear` during a loop, the SDK path, the run hold, a dead-pid hold
  (`Gone` in 24 ms), a crash (`Gone` in 36 ms); a status change reached a post in 36 ms (median). Verified live:
  `TaskStop` writes no notice and closes the task; a Monitor timeout writes only an `<event>` notice; a UI stop (tasks
  panel or SDK `stop_task`) writes the `killed` enqueue at once (M1 part b confirmed live). `wakeshot run` in a pseudo
  console: Ctrl+C, Ctrl+Break and closing the console behaved. Fixed m6: with `CLAUDE_CONFIG_DIR` set the status files
  are under that folder; new `claude_folder` and `ClaudeSource::in_folder`. For `p6-int-tray-mode`: build the source
  with `ClaudeSource::in_folder(claude_folder(&home, CLAUDE_CONFIG_DIR))`, not `ClaudeSource::new(home)`. Decided by
  the worker, for Haik's review: an empty `CLAUDE_CONFIG_DIR` counts as unset; the tray reads it from its own
  environment only (a per-shell value is not followed); a passed Monitor timer stays in the snapshot (the core drops it
  2 minutes after its time); pseudo console runs stand in for Windows Terminal. Left for Haik (`p5-haik-check`): the VS
  Code check, a real key press, the classic conhost window. Durable jobs are still off; the Esc cancel is still
  invisible, as known.
- 2026-11-04 02:12 (orchestrator, session `wakeshot-46`): CI on 4945be6 failed on ARM64 only:
  `restore::tests::a_tray_that_could_not_restore_says_what_is_still_changed` (`wakeshot-cli/src/restore.rs:456`). The
  fake tray sleeps a fixed 300 ms, then expects the request file; on a slow runner `restore` writes it later. Test-only
  fix (the fake tray waits for the file, with a timeout), routed to the next free build slot.
- 2026-11-04 02:28 (orchestrator, session `wakeshot-46`): the CLI restore test fix done and committed (Log in
  `p4-cli-restore`, "CI fix"): `fake_tray` waits for the request file (every 10 ms, up to 4 s, under the 5 s answer
  wait), then its delay; 20 of 20 runs passed; not yet run on the ARM64 runner. Seen by the worker, not changed:
  `run.rs` test `held_while_running` expects a `ping -n 2` child (about 1 s) still alive after the hold file shows; a
  very slow runner could miss that window (never seen failing).
- 2026-11-04 15:00 (orchestrator `wakeshot-27`): `p4-watch-tick-flake` done (from the ARM64 CI failure on 83b38fd,
  rerun green). The tick test ticks once and waits for its check, three times (ticks that come while one runs count
  once); the failed-post test's last tick the same; a second flake found under load (two `Menu(Restore)` in 1 of 8):
  the rig now writes data folder files atomically, as the CLI does. 20 serial runs and 24 of 24 load runs pass.
- 2026-11-06 15:35 (orchestrator, after a /clear): Round 1 of the bigger v1 started, four workers: `p4-contracts-r2`
  (alone among Rust cards: SourceId variants and stubs, `WantedFile::Query`, `Liveness::ImageSince`, `FindTail` for
  Codex reverted threads, whole-file reads of recently changed files, the `sqlite.rs` and `agent_hooks.rs` stubs),
  `p4-antigravity-research` and `p4-hooks-design` (docs only), `p4-live-harness` Part A (installs, logins, a ConPTY
  driver in `spikes/agent-drive/`, outside the workspace). Haik, about 15:30: Codex is installed and logged in (the
  orchestrator found the desktop app: `~/.codex`, no `codex` on PATH, no `sessions` folder yet); test prompts use
  minimum tokens. Orchestrator decision, for Haik's review: `p4-live-codex` may now run before `p4-live-copilot`
  (Codex is logged in; Copilot may need the sign-in sitting); both still run one at a time. Haik was told the
  optional steps before he went away (Copilot plan check, VS Code GitHub sign-in).
- 2026-11-06 ~16:00: `p4-antigravity-research` done (CLI and IDE: hooks only, free test yes, build in v1) and
  `p4-hooks-design` done (`docs/agent-hooks.md`). Security review of the design started (`wakeshot-reviewer`, report
  `docs/reviews/p4-hooks-design-security.md`); no hooks build before it. Orchestrator decisions on the design's three
  questions, for Haik's review (Haik away; each is the worker's default and easy to undo): 1. the Cursor CLI counts as
  not covered until a live test shows its plugin prompt and stop hooks fire (Cursor's code says its interactive loop
  skips plugin-only hooks; the editor fires them); 2. a Cursor or Antigravity approval wait holds, as Codex's does;
  3. hooks the user removed stay removed, with no menu item in v1. The hooks contracts go into a third contracts card,
  `p4-contracts-r3`, run alone after `p4-contracts-r2` and after the review.
- 2026-11-06 ~16:20: hooks design security review done (`docs/reviews/p4-hooks-design-security.md`): 0 blockers, 3
  majors, 10 minors. M1: an elevated agent runs the hook elevated, so its writes and prunes in a folder a Medium
  process can swap give that process an elevated delete; M2: removal trusts the marker's own paths (`..` not
  blocked), so a crafted marker deletes any file; M3: the Cursor editor's payload transport is not verified (the
  command-line form would put prompts in audit logs). The design worker fixes them; the same reviewer checks the
  majors; M3's check becomes the first step of `p4-cursor-source`, and Haik decides if the editor uses the command
  line.
- 2026-11-06 ~16:40: `p4-contracts-r2` committed (24f7405); `p4-opencode-source` Part B and `p4-vscode-source`
  running. The design worker fixed all 13 review findings but minor 13 (two lines in files it does not own:
  `product.md` "nothing that touches other apps" and the stale `research/cursor-gemini-signals.md`; both go to the
  README and docs card later). The design now has six contract items (`p4-contracts-r3` reads them from the design).
  The same reviewer checks the three majors (`docs/reviews/p4-hooks-design-security-verify.md`). Design question 5
  (a time cap on a hooked turn): orchestrator decision, no cap, by the product rule that every signal is checked
  against a fact, for Haik's review. Question 4 (if the Cursor editor puts prompts on the command line, does the
  editor hook ship; default no) goes to Haik only if `p4-cursor-source`'s first step finds that.
- 2026-11-06 ~17:00: hooks security verify done (`docs/reviews/p4-hooks-design-security-verify.md`): majors 1 and 2
  Fixed, major 3 Partly (the Cursor editor and CLI load one plugin folder); 3 new minors. The design worker applied
  all four: if the Cursor editor can pass hook input on the command line, Wakeshot adds no Cursor plugin at all
  (checked in the editor's code and at each hook run); the Medium copy re-checks its values; a 2 s deadline; random
  temp names. Contract item 2 gains `command_line_contains`. Found outside scope by the design worker, for a later
  fix card: an elevated `wakeshot run` and an elevated uninstall can still meet a junction in Wakeshot's own data
  folder (existing code, not the hooks). `p4-contracts-r3` may start; it waits for the running Rust cards to drain.
- 2026-11-06 ~17:05: `p4-live-harness` Part A done (bb53377). Installed for Haik's user at Medium, no admin, for his
  review (remove commands in the card's Result): Copilot CLI 1.0.106, OpenCode 2.2.5, Codex CLI 0.167.0 (npm), Ollama
  0.40.15 with `qwen3:4b-instruct` (adds `Startup\Ollama.lnk`), Cursor 3.27.12 and its CLI 2026.11.05, Antigravity
  CLI 1.5.2 (puts `%LOCALAPPDATA%\agy\bin` first in the user PATH) and IDE 2.6.0. Logins reused, no token passed:
  Codex CLI through the desktop app's login, Copilot CLI through `gh` on Copilot Free, VS Code Copilot signed in on
  Free. Haik's sign-in sitting (about 10 min): Cursor and Antigravity only. Orchestrator decision: harness Part B now
  (small, its own file), then `p4-live-codex` at once, ahead of `p4-contracts-r3`, since the live lane is the longest
  chain; `p4-contracts-r3` runs at the next drain. For the live cards: a folder listing showed a stale size and time
  for a file another process held open, so `FindNewest` and `Recent` may miss writes until a read or a close (time
  it); the VS Code `copilot` shim stays as a parent, so one Copilot session has two `copilot.exe` images.
- 2026-11-06 ~17:30: Haik did the sign-in sitting (Cursor editor and CLI, Antigravity CLI and IDE): "Done
  everything", no step reported as different. `p4-live-codex` running (Round 3, no Haik). Every v1 agent can now be
  tested live by agents.
- 2026-11-06 ~17:45: `p4-vscode-source` done (5869c5c). Haik asked for a card to rework the video and `README.md` so
  the focus is every AI agent, not Claude Code ("ok that the example is on Claude Code in the video, but the tray will
  have a new look (AI apps)"): `docs/tasks/p6-agents-video-readme.md` (Part A a plan with stills for his yes, Part B
  after the live tests). It also takes the Round 4 README and `site/` list, the `product.md` README entry and the
  hooks review minor 13. One question to Haik on what "new look (AI apps)" means.
- 2026-11-06 ~18:10: OpenCode database read review done (`docs/reviews/p4-opencode-db-read.md`): 0 blockers, 1 major,
  7 minors. M1: after OpenCode dies without closing the database, its `-wal` and `-shm` stay, and each Wakeshot read
  rewrites `-shm`, whose change notice starts the next read: about three reads a second until OpenCode opens and
  closes cleanly (the reviewer's probe confirmed it). The fix is watch plumbing (re-run only on `<db>-wal` or `<db>`
  notices), so it goes to `p4-contracts-r3`, which runs alone; `p4-live-codex` may edit `watch/agents.rs` now. The
  minors in the OpenCode card's own files go to its worker now (shim refuses deletes, table test gaps, no checkpoint
  on close). The same reviewer checks the major after `p4-contracts-r3`.
- 2026-11-06 ~18:25: `p4-live-codex` done (a1ca2ce): every step run by the agent at Medium, 7 one-line prompts
  (gpt-6-luna, low). Restart Manager names the session's own `codex.exe` (TUI and `exec`); an approval wait held;
  Esc writes `turn_aborted`; `FindNewest` followed a reverted thread live; a crash posted `Gone` in 115 ms (the stale
  lock stays until the next Codex start deletes it). Fix: the source now lists `sessions\` one level deep so the
  tree is watched; before it, a turn reached a post only at the 30 s tick and a 12 s turn was never seen. Codex is
  in the tray (`CODEX_HOME`, else `.codex`). Worker calls for Haik's review: that listing workaround instead of a
  contract change; each notice walks all date folders; a background terminal left running after Esc does not hold;
  `CODEX_HOME` from the tray's own environment only. Track note 7 updated. Next: `p4-contracts-r3` runs alone once
  the OpenCode minor fixes end; then `p4-cursor-source`, `p4-agent-hooks-install` and `p4-live-copilot` in parallel.
- 2026-11-06 ~18:50: Haik asked why the agents are not integrated in parallel. Orchestrator decisions, for speed and
  easy to undo: live cards run in parallel from now on (at most four building cards in all). `p4-contracts-r3`
  (running alone) also wires Copilot, VS Code and OpenCode into `tray_mode.rs` and adds `agent-watch` rows for VS Code
  and OpenCode; `p4-antigravity-source` wires Cursor and Antigravity; no live card edits those two files. The
  `--dry-run` app step takes turns through `<scratchpad>\dry-run.lock`. The hooks code review runs beside the Cursor
  and Antigravity live cards instead of before them; those cards first copy the folder they write into
  (`.cursor\plugins\`, `.gemini\config\plugins\`) to the scratchpad. Order after `p4-contracts-r3`: `p4-cursor-source`,
  `p4-agent-hooks-install`, `p4-live-copilot`, `p4-live-vscode` (four), then `p4-live-opencode` when a slot frees and
  the OpenCode major is verified; then `p4-antigravity-source` beside the hooks code review; then `p4-live-cursor`
  and `p4-live-antigravity` together. Estimate told to Haik: about 22:00 to 22:30 (a guess).
- 2026-11-06 ~19:35: `p4-contracts-r3` done (98c8795): whole-workspace checks green on x64 and ARM64 (1622 tests),
  one `windows` feature (`Wdk_System_Threading`, missed by the design); `image_path` and `command_line_contains` take
  a `ProcessRef`. Pushed 98c8795 (25 commits, secret scan clean). Started: `p4-cursor-source`,
  `p4-agent-hooks-install`, `p4-live-copilot`, `p4-live-vscode`, and the OpenCode major verify. Haik may use the
  laptop for normal work; he was told to keep away from the AI apps, sleep and the lid until the live tests end.
- 2026-11-06 ~19:45: OpenCode review verify (`docs/reviews/p4-opencode-db-read-verify.md`): the major is Fixed; the
  minor fixes opened nothing new. Left, bounded: leftover files after a crash still get three reads per 30 s tick;
  `p4-live-opencode` runs that case. The OpenCode database read review has passed. `p4-live-opencode` starts when a
  build slot frees (four building now).
- 2026-11-06 ~19:55: found at the orchestrator's CI check: CI's `core-linux` job has failed since fb0ebce (the last two
  pushes of the session before this one, and 98c8795): 15 of 22 `codex_source.rs` tests hard-code Windows path
  strings, which fail where `Path::join` uses `/`; cargo stops at the first failing test binary, so later core test
  files never ran on Linux. The Windows jobs pass. `CLAUDE.md` "Checks" has no Linux core run, so local checks could
  not catch it. Fix card `p4-core-linux-tests` started (owns the Codex and OpenCode test files; tries a Rust toolchain
  in WSL for Haik's user, no sudo; lists other failing files). Orchestrator decision: it runs as a fifth Rust card,
  since it builds only the core and CI is red. The running cards were told the path rule.
- 2026-11-06 ~20:15: `p4-core-linux-tests` done (750b023, 322924e): CI's core step now runs `--no-fail-fast`. From
  `p4-live-vscode`: in this VS Code every new chat from `code chat` goes to the Copilot (agent host) harness, not
  Local, even with `chat.defaultToCopilotHarness: false`; the session is in `~\.copilot\session-state\<id>\` owned by
  `copilot-runtime.exe`, and the Copilot source posts it working but `Gone` (its liveness looks for `copilot.exe`), so
  it does not hold. Routed to `p4-live-copilot` (research rule 8: accept `copilot-runtime.exe` and `Code.exe` owners).
  Also `alive=Gone` from Medium for a running `copilot.exe`: elevation of the test process or a real Medium-to-High
  gap; `p4-live-copilot` says which. The VS Code worker was asked to check who started the elevated VS Code.
- 2026-11-06 ~20:30: `p4-agent-hooks-install` done (f46828f). Not wired yet: the tray's `startup::add_agent_hooks`
  call (added to `p4-antigravity-source`) and the CLI's `hook <agent> remove` call (sent to `p4-cursor-source`).
  Worker's question for Haik (a Cursor uninstall and reinstall reads as "the user removed the hooks", so they never
  come back): orchestrator decision, for Haik's review, easy to undo: a whole agent folder gone is the agent going
  away, so its record is forgotten and the hooks come back when it is found again; only Wakeshot's own folder gone
  stays "removed by the user" (`p4-antigravity-source` item 7). `p4-live-opencode` started (slot freed). The VS Code
  live card asked Haik to close its elevated test window after one approval click (relayed in chat).
- 2026-11-06 ~20:40: `p4-live-opencode` found a watch bug: OpenCode turns are read only at the 30 s tick, never on a
  notice (a 2.8 s turn was never posted). The `-wal` keeps one size and NTFS reports no last-write change while
  OpenCode holds it open, so no notice fires; and OpenCode leaves `-wal` and `-shm` after a clean stop, so the "only
  while `-wal` exists" rule never turns reads off. Orchestrator decision: a poll of about 2 s for `Query` wants while
  a process of the source's images lives, none while none lives, through a new field on `WantedFile::Query` (a narrow
  contract change: only OpenCode and the watch thread name `Query`), built by the `p4-live-opencode` worker after
  `p4-live-copilot` (owner of `watch/*` now) ends, with a live re-time. For Haik's review: this narrow change runs
  beside cards that never name `Query`, not alone (his rule says contracts change alone; the risk is only to the
  files that name the variant).
- 2026-11-06 ~20:50: `p4-cursor-source` done (f9a41e2): the privacy Gate holds (temp file on every Cursor path that
  runs a plugin hook), so no design Question 4 for Haik. Worker calls for Haik's review: the runtime check searches
  `conversation_id` without quotes (Node escapes quotes); an elevated hook writes no log line; a bad or newer state
  file gives no notice. Started: `p4-antigravity-source` and the hooks code review (`wakeshot-reviewer`,
  `docs/reviews/p4-hooks-code.md`), in parallel, as decided at 18:50.
- 2026-11-06 ~21:00: `p4-live-copilot` done (00e0120). The Gone bug was not elevation: nobody holds the `.lock`; the
  inner `copilot.exe` (or VS Code's `copilot-runtime.exe`) holds `inuse.<pid>.hold`, so `copilot.rs` names the `.hold`
  and `watch/agents.rs` counts the holder whose pid is in the file name, whatever its image (only that pid when
  Restart Manager fails). VS Code's Copilot harness chats now hold as "Copilot". Worker calls for Haik's review: the
  pid-in-name rule; those chats show as "Copilot". `watch/*` handed to the `p4-live-opencode` worker for the poll.
