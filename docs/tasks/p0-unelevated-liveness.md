# Check that an unelevated process sees elevated Claude Code sessions

Phase: 0. Status: done. Needs Haik: no. Agent: `wakeshot-worker`.
Runs after: `p0-claude-live-tests` (done). Parallel with: `p0-spike-safety-fixes`.

## Goal

Wakeshot runs unelevated (Medium integrity). Haik's Windows Terminal elevates by default, so his Claude Code sessions
run at High integrity. All earlier Claude tests ran elevated. Find out whether a Medium process can do every read and
liveness check the design needs against elevated Claude sessions.

## Read first

- `docs/reviews/p0-architecture.md`: the finding about elevated liveness checks.
- `docs/tasks/p0-claude-live-tests.md` Result, and the "Claude Code" part of `docs/research.md`.
- `spikes/claude-watch/` (the watcher from the live tests).
- `docs/architecture.md`: the Claude Code source part.

## Deliverables

- Facts in `docs/research.md` with how each was checked, for a Medium-token process against a High-integrity
  `claude.exe` session: read `~/.claude/sessions/<pid>.json`; open the PID for liveness (which access rights work:
  `PROCESS_QUERY_LIMITED_INFORMATION`, `SYNCHRONIZE`, wait on the handle); read the process start time or image path
  to guard against PID reuse; read the transcript `projects\*\<sessionId>.jsonl`; get file change notifications on
  the sessions folder.
- Also the reverse where it matters: a Medium Claude session seen from a Medium process (the normal case for other
  users).
- If a check fails at Medium, the simplest workaround and its cost, as input for the design.
- Any change to `spikes/claude-watch/` needed to run these checks, with tests.

## Acceptance checks

- Each fact names the integrity level of both sides, checked from the token, not assumed.
- `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` pass in `spikes/claude-watch/` if you change it.

## Out of scope

- Power settings: change none.
- Never read the `.key` files next to the session files. Never write to `~/.claude` except what a test session
  itself writes.
- Do not stop or change any Claude session you did not start. The orchestrator session and PID 32332 are not yours.
- `spikes/power-settings/`, `spikes/lid-sleep/`, `spikes/tray/` (the parallel card changes them).

## Questions for Haik

## Found outside scope

- Side effects of my test sessions, written by Claude Code itself: two transcript folders in `~/.claude/projects`
  (`...-scratchpad-unelevated-liveness-cc-high` and `...-cc-medium`, one file each). Not deleted (outside the repo
  and my scratch folder). Harmless; delete them if unwanted.
- Ten `cmd` windows opened and closed on the desktop (Explorer launches of the `.cmd` files). Nothing left open.
- `spikes/claude-watch/src/procs.rs` `verdict` reports `0x80070005` (no access) and `0x80070057` (gone) both as
  "dead or no access". Fine for the spike, but the product must not count a denial as dead (see Result). Not changed.
- `docs/research/claude-code-signals.md` rule 2 and the review's finding 2 still say "confirmed only from an elevated
  process". Now confirmed from Medium too; not edited here (outside the card's file list).

## Log

- 2026-11-03: card written by the orchestrator from the early architecture review.
- 2026-11-03: worker started. Reading the review, live-test card, research and the watcher.
- 2026-11-03 03:33: plan: add a one-shot `--probe` mode and a `--wait-exit PID` mode to `spikes/claude-watch`; run them
  at Medium from a `.cmd` opened with `explorer.exe` (the lid spike method, no parent spoofing) and elevated from this
  shell. Targets: orchestrator 20636 and pid 32332 (read only), plus my own test sessions over stream-json (one High,
  one Medium). Scratch folder `<scratchpad>\unelevated-liveness`. From the elevated side: `icacls` shows the sessions
  folder, `20636.json`, `32332.json` and `projects` with only inherited ACEs (SYSTEM, Administrators, haika: full) and
  no mandatory label.
- 2026-11-03 03:35: `--probe` built (23 tests, fmt and clippy clean). Run elevated (`probe-high-1.log`) and at Medium
  through Explorer (`probe-medium-1.log`, `whoami` in the same `.cmd`: Medium Mandatory Level) against 20636 and
  32332, both High. Medium: `QUERY_LIMITED_INFORMATION`, `SYNCHRONIZE` and both together open; `QUERY_INFORMATION`,
  `VM_READ`, `READ_CONTROL` are denied (0x80070005). Start time equals `procStart`, image path, exit code, 1 s wait,
  target token (High), system process list, status files and transcripts all work. No change events came in the 5 s
  window (no session wrote); a dynamic test with my own sessions comes next.
- 2026-11-03 03:37: own High test session (stream-json, haiku, driver pid 19288, claude pid 32484) and an elevated
  `pwsh Start-Sleep` stand-in for an elevated `wakeshot run` (pid 17588). Medium `--wait-exit 32484` (from Explorer)
  opened the handle; one prompt; driver stopped by PID at 03:37:24.395; the Medium wait signaled at 03:37:24.791, exit
  code 0, `32484.json` removed. Medium probe (`probe-medium-2.log`): same results for 32484 and 17588 as for 20636;
  change notifications on the sessions folder arrived (2 modified and 1 removed on `32484.json`, 1 on another file).
  Probe now logs each notification as it arrives, to show the delay.
- 2026-11-03 03:40: Medium test session started by Explorer (driver 27108, claude 31016; `whoami` in the `.cmd`:
  Medium). Medium probe saw its file added and each status write; all six open rights work Medium to Medium and
  High to Medium. Medium `--wait-exit 31016` signaled 0.57 s after the driver was stopped by PID.
- 2026-11-03 03:42: hard kill test: own High session 32704 (driver 34448), Medium `--wait-exit` open, then
  `Stop-Process -Force 32704`: signaled after 24 ms, exit code 0xFFFFFFFF, `32704.json` left behind. Medium probe on
  it: every open fails `0x80070057`, pid not in the system list. A third own session (34224, driver 7068) started and
  the leftover was gone at once; that session ended by stopping its driver.
- 2026-11-03 03:44: own High session 6228 (driver 33616) under a Medium notify probe: the `idle` write
  (`updatedAt` 03:44:36.544) came in at 03:44:36.549; removal 8 ms after the driver stop. Sleeper 17588 stopped by
  PID. No test process left (checked by command line). Sessions folder back to `20636.json` and `32332.json`.
- 2026-11-03 03:46: logs copied to `spikes/claude-watch/out/unelevated-liveness/` with the user SID masked. Facts
  written to `docs/research.md`. Final fmt, clippy, tests pass.

## Result

Done. A Medium (unelevated) process can do every read and liveness check the design needs against High (elevated)
Claude Code sessions. No workaround needed for Haik's setup. Full facts: `docs/research.md`, "Reading elevated Claude
Code sessions from an unelevated process".

What was checked, Medium reader against High `claude.exe` (levels read from both tokens, plus `whoami` in the same
`.cmd`; reader started by Explorer, no parent spoofing):

| Check | Result |
|---|---|
| Read `sessions\<pid>.json` | ok (no label on the file, user has full control) |
| `OpenProcess` `QUERY_LIMITED_INFORMATION`, `SYNCHRONIZE`, both | ok |
| `OpenProcess` `QUERY_INFORMATION`, `VM_READ`, `READ_CONTROL` | denied `0x80070005` |
| Start time (`GetProcessTimes`) vs `procStart` | equal |
| Image path (`QueryFullProcessImageNameW`) | ok |
| Wait on the handle | ok; signals 0.4 s after a clean exit, 24 ms after a hard kill |
| Target token (`OpenProcessToken` `TOKEN_QUERY`) | ok, shows High |
| System process list create time (no handle) | ok, equals `procStart` |
| Transcript `projects\*\<sessionId>.jsonl` | ok |
| `ReadDirectoryChangesW` on the sessions folder | ok; added, modified, removed; 5 ms after the one write I timed |
| Pid of a killed session | every open fails `0x80070057`; not in the system list |

Medium to Medium (the normal case) and High to Medium: all six rights open, every check works. An elevated non-Claude
process (stand-in for an elevated `wakeshot run`) behaves the same as an elevated `claude.exe`.

Input for the design:

1. Open session processes with exactly `PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE`. One more right
   (for example `QUERY_INFORMATION`, which some process crates ask for) fails the whole open on elevated sessions.
2. Treat `0x80070057` as gone. Treat `0x80070005` as "no access", never as dead: fall back to the system process list
   (create time equals `procStart` means alive), polled at the safety tick. Cost about 5 to 9 ms per call, and an
   exit is then seen only at the next poll. Only expected when elevation uses another account (not verified).
3. Wait on the process handle for exit and use `ReadDirectoryChangesW` for the sessions folder: both work unelevated.

Files changed:

- `spikes/claude-watch/src/probe.rs` (new): `--probe` and `--wait-exit` modes, 7 tests.
- `spikes/claude-watch/src/main.rs`: the two modes and their flags.
- `spikes/claude-watch/src/watch.rs`: `find_transcript` moved to a free function, so the probe can use it.
- `spikes/claude-watch/src/procs.rs`: `filetime_u64` made `pub`.
- `spikes/claude-watch/Cargo.toml`: `windows` features for tokens, security, files, IO, the system process list.
- `spikes/claude-watch/out/unelevated-liveness/*.log` (new): the 11 run logs, user SID masked.
- `docs/research.md`: new section with the facts and how each was checked.

Acceptance checks:

- Each fact names the integrity level of both sides, read from the token: pass (every log starts with the reader's
  token; each target line shows the target's token).
- `cargo fmt --check`, `cargo clippy -- -D warnings` (also `--all-targets`), `cargo test` (23 tests): pass.

Open items:

- Not verified: elevation with another account's password, sessions in another Windows session, the Desktop app.
- Orchestrator: consider marking review finding 2 and rule 2 in `docs/research/claude-code-signals.md` as confirmed
  from Medium.
