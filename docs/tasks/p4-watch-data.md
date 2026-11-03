# Watch thread: the data folder (holds, config, restore request)

Phase: 4. Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 70K tokens.
Runs after: `p4-hold-files`, `p4-watch-reads`. Parallel with: every other card of this track and every card of tracks
2, 3, 5, 6.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-watch-data\target`. Uses the real laptop: no (temp folders, fakes).

## Goal

Turn the data folder into events: `holds\*.json` into `Event::Holds` (with liveness), `config.json` into
`ConfigChanged`, and `restore-request.json` into one `Menu(Restore)` per appearance.

## Read first

- `docs/phases/04-sources.md`: track notes 10, 14, 15, 16.
- `docs/architecture.md` section 4 (watch row, "How they talk"), section 5 "Liveness", section 6 (file table, hold
  format, version rule), section 8 (Liveness Unknown).
- `crates/wakeshot-core/src/holds/file.rs` (from `p4-hold-files`), `files.rs`, `event.rs`.
- From the contracts card: `store::DataDir` (paths), `process::Liveness` and `FakeLiveness`, the log seam and its fake.

## Deliverables

`crates/wakeshot-windows/src/watch/data.rs`, driven by calls (no thread, no folder watch of its own). It reads every
data file (`config.json`, `restore-request.json`, hold files) through `wakeshot_core::files::read` (the version rule,
first JSON value only, byte order mark skipped; `p1-contracts-split` edit 6):

1. `holds(dir, liveness) -> HoldsRead`: list `holds\*.json` (temp names never end in `.json`), read each whole,
   `holds::file::parse`; a bad read or parse is retried once 200 ms later (the caller schedules it: return the names to
   retry); still bad, newer version, or bad `expires`: left out and named for one log line per file and change. For
   each good file: `hold_alive` from `liveness.start_of_pid(pid)` and the file's last write time (as a `FileTime`), then
   `facts`. The result is the full list for `Event::Holds`.
2. `config(dir) -> Option<Event>`: missing file gives `ConfigChanged(Config::default())`; newer version or bad file
   gives no event and a log line (the tray keeps what it has); only a change from the last posted value gives an
   event.
3. `restore_request(dir) -> Option<Event>`: `Menu(MenuCommand::Restore)` when the file appears; nothing while it stays;
   armed again once it is gone. A newer version still counts (the request has no fields to misread); a bad file counts
   too (the CLI wrote it).
4. Never deletes or writes a file: deleting a hold whose process is gone is the core's decision (`DeleteHold`), done
   by the IO thread; the request file is deleted by the IO thread after `state.json` (track note 15).

Tests first, in a temp data folder with `FakeLiveness`: each hold kind; `procStart` match, mismatch, missing (start
before and after the file's write time); a dead pid (`Gone`), an unknown lookup (`Unknown`, file kept in the list);
`expires` past and future (the core decides; the facts carry it); a bad file retried once; a newer file; a temp name;
config missing, changed, unchanged, newer; the restore request across appear, change notices, disappear, appear.

## Acceptance checks

- `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-windows --all-targets --target
  x86_64-pc-windows-msvc -- -D warnings` and the same for `aarch64-pc-windows-msvc`; `cargo test --locked -p
  wakeshot-windows`: clean as the check rule below says, new tests named in the Result.
- No test touches `%LocalAppData%\Wakeshot` (all paths come from a `DataDir` on a temp folder).
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- The thread, folder watch and exit waits (`p4-watch-thread`). Writing any file (CLI cards, IO thread). The real
  `process::Liveness` (power track). Any file not listed above.

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03 16:53: card written by `p4-plan-sources`.

## Result

What was done, files changed, checks and outcomes, open items.
