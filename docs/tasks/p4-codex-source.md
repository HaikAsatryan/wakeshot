# Codex CLI source

Phase: 4. Status: later (after v1, Haik 2026-11-03; `docs/phases/02-06-lighter.md`). Needs Haik: no (his live test is `p4-live-codex`). Agent: `wakeshot-worker`. Size: about 70K
tokens.
Runs after: the contracts card (it creates `sources/codex.rs` and adds `WantedFile::Names`, `FindTail`,
`Feed::Present`). Parallel with: every other card of this track and every card of tracks 2, 3, 5, 6.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-codex-source\target`. Uses the real laptop: no.

## Goal

`CodexSource`, an `AgentSource` built from the Codex source code reading in `research/other-agent-signals.md`:
sessions from thread lock files, activity from the last turn event of the rollout. Not tested live until
`p4-live-codex`; the docs mark it so.

## Read first

- `docs/phases/04-sources.md`: track notes 7, 8, and Question 1 (answered with the default, Haik 2026-11-03 18:09:
  an open turn holds).
- `docs/architecture.md` section 7 ("Codex CLI and Copilot CLI").
- `docs/research/other-agent-signals.md`: "OpenAI Codex CLI" (all of it), "Design notes for Wakeshot", "Fixture
  sketches".
- `crates/wakeshot-core/src/sources.rs` (after the contracts card).

## Deliverables

1. `crates/wakeshot-core/src/sources/codex.rs`: `CodexSource::new(home: PathBuf)` where `home` is `CODEX_HOME` if set,
   else `%USERPROFILE%\.codex` (the caller resolves it). `wanted()`: `Names { dir: <home>\thread-writer-locks, depth:
   0, accept: <thread id>.lock }` (lock files are never opened: Codex holds an exclusive lock and treats a second
   holder as an error); for each present lock, `FindTail { root: <home>\sessions, depth: 3, name_suffix:
   "-<thread id>.jsonl" }` (`YYYY\MM\DD`). Skip `.jsonl.zst`. A forked rollout (`..._<rollout id>.jsonl`) is matched
   too if the live test shows it carries the same thread; until then not matched (say so in the Result).
2. Activity from the last turn event in the rollout: `task_started` (or `turn_started`) gives `Working`;
   `task_complete`, `turn_complete` or `turn_aborted` gives `Idle`; no turn event yet, or no rollout found yet
   (created at the first item), gives `Idle`. Approval waits are not on disk, so they read as `Working` (Question 1).
   Lines of other kinds are ignored; a line that is not JSON is skipped; an `event_msg` turn event without `type` or
   `turn_id` gives `UnknownValue("rollout", <kind>)`.
3. `SessionFacts`: `key.id` the thread id, `liveness: AnyProcess { image: "codex.exe", lock_file }` (the watch thread
   ties it to the holder, track note 7), `label: "Codex"`, `timers: []`, `open_tasks: 0`. Lock file gone: the
   session is gone.
4. Fixtures `crates/wakeshot-core/tests/fixtures/codex/` from the research sketches (running, complete, aborted, the
   `turn_*` aliases, a rollout before any turn, junk lines), each marked "built from the schema, not a real log" in a
   README. Tests first: `crates/wakeshot-core/tests/codex_source.rs` feeds lock names and rollout bytes (whole and in
   pieces) and checks the facts.
5. The module doc says "not tested live" until `p4-live-codex` changes it. `docs/research/other-agent-signals.md`
   belongs to the live cards; do not edit it here.
6. Moved here from `p4-watch-reads` when Codex and Copilot went after v1 (Haik, 2026-11-03): deliverable 2 of that
   card, `crates/wakeshot-windows/src/lockowner.rs` (Restart Manager `holders(path)`), with its tests and the
   research line on the Restart Manager answers. Size about 25K more.
7. Moved here from `p4-watch-agents` (done by then): the `AnyProcess { image, lock_file }` liveness in
   `crates/wakeshot-windows/src/watch/agents.rs` through `lockowner::holders`, as that card's deliverable 3 first
   said (cache until the lock file changes or the holder is gone; no holder gives `Gone`; a Restart Manager error
   falls back to "any live process with that image" while the lock file is `Present`; a failed process list gives
   `Unknown`), and its tests. Size about 20K more.

## Acceptance checks

- `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-core --all-targets --target
  x86_64-pc-windows-msvc -- -D warnings` and the same for `aarch64-pc-windows-msvc`; `cargo test --locked -p
  wakeshot-core`: clean as the check rule below says, new tests named in the Result.
- No code path opens or reads a lock file (the source never asks for one with `Folder` or `Tail`).
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- Restart Manager and process checks (`p4-watch-reads`, `p4-watch-agents`). Goals (track note 8). The app-server
  daemon beyond what the lock files show. The live test (`p4-live-codex`). Any file not listed above.

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03 16:53: card written by `p4-plan-sources`.

## Result

What was done, files changed, checks and outcomes, open items.
