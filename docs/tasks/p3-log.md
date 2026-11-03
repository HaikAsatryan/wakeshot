# Log: the shared log writer

Phase: 3. Status: ready. Needs Haik: no. Real laptop: no. Agent: `wakeshot-worker`. Size: about 80K tokens.
Runs after: Haik's sign-off of the track plans; the contracts card. Parallel with: every other wave 1 card of this
track and any card of tracks 2, 4, 5, 6.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p3-log\target`.

## Goal

Fill `log`: one plain-text log that every Wakeshot process appends to, one line per event, with lines from different
processes never mixed. Tracks 4, 5 and 6 use it.

## Read first

- `CLAUDE.md`; `docs/phases/03-windows-power.md` (track notes 1 to 3, 9).
- `docs/architecture.md`: section 8 "Log" (line content, roll, what is never logged), section 6 (data folder).
- `docs/research.md`: "The spike's log clock drifted 16 s"; "Power track plan checks" (local time).
- The stub `crates/wakeshot-windows/src/log.rs` from the contracts card (trait `Log`, `LogWriter`, `Proc`, `Level`,
  `MemoryLog`): its signatures are the contract.

## Deliverables

`crates/wakeshot-windows/src/log.rs`, filled in, with its tests:

- Line: local wall time with milliseconds and its UTC offset, monotonic seconds (`clock::now().mono_ms`), process
  (`tray`, `guard`, `restore`, `cli`, `hook`), level, area, text. Wall time read for every line
  (`GetSystemTimePreciseAsFileTime`, `FileTimeToLocalFileTime`), never derived from a start time. CR and LF in the text
  become spaces, so one call is one line and a hold's `reason` cannot forge a line.
- `LogWriter::open(logs_dir, proc)`: creates the folder; a handle opened for append (`FILE_APPEND_DATA`, read, write
  and delete sharing); one `WriteFile` per line. A failed write drops that line; the writer goes on.
- `LogWriter::roll(logs_dir)`: run by the tray at start; when `wakeshot.log` is over 1 MB, `wakeshot.1.log` becomes
  `wakeshot.2.log`, `wakeshot.log` becomes `wakeshot.1.log`; at most three files (track note 9).
- `MemoryLog` stays as the contracts card wrote it (other tracks use it).

## Acceptance checks

Tests in `log::tests`:

- `line_format` (a pure formatter on fixed inputs, positive and negative offsets, CR and LF stripped).
- `appends_from_many_processes_never_mix`: the test runs its own test exe as 4 children (an environment variable picks
  the child role), each appending 2000 lines of different lengths at once; every line in the file is whole and each
  child's count is right. This answers section 8's "not verified (Phase 3)"; record it in `docs/research.md`.
- `roll_keeps_three_files`, `roll_with_file_open_elsewhere` (another handle open: the roll works or returns `Err`; no
  line lost).
- `local_offset_matches_windows`: the offset equals the one `Get-Date -Format zzz` prints (compare once by hand, record
  in the Log and research).

Commands: `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-windows --all-targets` with
`-D warnings` for both targets; `cargo test --locked -p wakeshot-windows log::`. Record each outcome.

- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- What each process logs (each card logs its own lines), "Open log" (tray track), the never-logged list (each caller).
- Any file but `log.rs`.

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03 17:00: card written by `p3-plan-windows-power`.

## Result

What was done, files changed, checks and outcomes, open items.
