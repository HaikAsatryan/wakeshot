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
Notes 7, 8 and 9 apply after v1: Codex and Copilot moved there (Haik, 2026-11-03).

7. **Codex and Copilot: the lock file ties a session to its process.** The watch thread asks Restart Manager which
   process holds the lock file open and checks that pid and start time. Only if the live test shows Restart Manager
   cannot see the holder does the weaker rule stay ("any `codex.exe` alive and the lock file exists", marked "can
   outlive its session" in the docs).
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
| `p4-codex-source` | no | contracts card | later (after v1, Haik 2026-11-03) |
| `p4-watch-folders` | no | contracts card | done (00:26): 20 tests, no handle leak in 1,000 cycles |
| `p4-watch-reads` | no | contracts card | done (21:12): 18 tests; `names` returns `io::Result` |
| `p4-cli-args` | no | contracts card | in progress |
| `p4-claude-transcript` | no | `p4-claude-status`, `p4-cron`, `p4-claude-live-shapes` | done (22:35): 46 tests; no contract change |
| `p4-copilot-source` | no | `p4-cron` | later (after v1, Haik 2026-11-03) |
| `p4-watch-data` | no | `p4-hold-files`, `p4-watch-reads` | done (21:12): 19 tests |
| `p4-watch-agents` | no | `p4-watch-reads` | done (22:43): 19 tests; API changes for `p4-watch-thread` in its Result |
| `p4-watch-thread` | no | `p4-watch-folders`, `p4-watch-data`, `p4-watch-agents`, `p3-process` | in progress |
| `p4-cli-run` | no | `p4-cli-args`, `p4-hold-files`, `p3-store`, `p3-process`, `p3-log` | in progress |
| `p4-cli-commands` | no; screen once (opens one file in the default app; global lane, review M6) | `p4-hold-files`, `p3-store`, `p3-process`, `p3-log` | done (00:36): 30 tests; screen step ran once |
| `p4-cli-restore` | no (Phase 1 Question 12: the default, Haik 2026-11-03 18:09; nothing to add) | `p3-journal`, `p3-power-plan`, `p2-ownership-restore`, `p3-store`, `p3-process`, `p3-log` | done (00:36): 10 tests |
| `p4-live-claude` | optional: 3 minutes for the VS Code check; agent-session lane | `p4-watch-thread`, `p4-claude-transcript`, `p4-cli-run`, `p3-process`, the Claude parsing review (`docs/phases/02-06-lighter.md`) | ready |
| `p4-live-codex` | yes: about 20 minutes (install or sign in, then type the steps) | `p4-live-claude` | later (after v1, Haik 2026-11-03) |
| `p4-live-copilot` | yes: about 20 minutes (install or sign in, then type the steps) | `p4-live-codex` | later (after v1, Haik 2026-11-03) |

Bundles (one worker, both cards in order; `docs/phases/02-06-lighter.md`): `p4-cli-args` + `p4-cli-run`;
`p4-cli-commands` + `p4-cli-restore`; `p4-watch-reads` + `p4-watch-data`. Codex and Copilot moved to after v1 (Haik,
2026-11-03): their four cards are `later`, and the lock-holder code moved from `p4-watch-reads` and
`p4-watch-agents` into `p4-codex-source`.

"Contracts card" is the one card the orchestrator runs alone before the builds (this track's changes are in the
Result of `docs/tasks/p4-plan-sources.md`). `p3-*` cards are the power track's (`docs/phases/03-windows-power.md`);
`p3-store`, `p3-process` and `p3-log` are in its first wave.

Cards that use the real laptop (review M6, m19):

- Agent-session lane: `p4-claude-live-shapes`, `p4-live-claude`, and after v1 `p4-live-codex` and `p4-live-copilot`, start agent
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
| `p4-codex-source` (later) | `crates/wakeshot-core/src/sources/codex.rs`; `crates/wakeshot-core/tests/codex_source.rs`; `crates/wakeshot-core/tests/fixtures/codex/`; `crates/wakeshot-windows/src/lockowner.rs`, and the `AnyProcess` part of `watch/agents.rs` (after `p4-watch-agents`) |
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
5. After v1: `p4-codex-source`, `p4-copilot-source`, then `p4-live-codex`, `p4-live-copilot`, one at a time and with
   Haik.

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
