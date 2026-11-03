# Log: the shared log writer

Phase: 3. Status: done. Needs Haik: no. Real laptop: no. Agent: `wakeshot-worker`. Size: about 80K tokens.
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

None.

## Found outside scope

Problems seen but not fixed.

- `docs/architecture.md` section 8 still says that lines from many processes never mix is "not verified (Phase 3)".
  It is now checked (`docs/research.md`, "Log writer checks"); the line should point there. Not this card's file.
- `docs/research.md` "Power track plan checks" says `Win32_System_Time` is not declared; it is now. Research is
  additions only, so a new line in "Log writer checks" says so; the old line stays.

## Log

- 2026-11-03 17:00: card written by `p3-plan-windows-power`.
- 2026-11-03: worker started; read CLAUDE.md, plan "How we work", the card, the `log.rs` stub.
- 2026-11-03: read architecture 6 and 8, track notes, research. Checked `windows` 0.62.2 signatures in the local
  registry (`CreateFileW`, `WriteFile`, `MoveFileExW`, `GetFileSizeEx`, `FileTimeToLocalFileTime`,
  `GetDynamicTimeZoneInformation`; `HANDLE` is not `Send`, so the writer keeps a std `OwnedHandle`). Read Microsoft's
  `FileTimeToLocalFileTime` and "File Access Rights Constants" pages. `Get-Date -Format zzz` here: `+04:00`. Design
  picked (see Result); writing `log.rs`.
- 2026-11-03: `log.rs` written and formatted (`rustfmt --edition 2024`, only this file; it declares no child
  modules). First `cargo test --locked -p wakeshot-windows log::`: 8 passed, 1.6 s.
- 2026-11-03: second run fails to compile in `wakeshot-core/src/ownership/fake.rs` (missing `in_force_by`,
  `stored_by` in `FakePlan`): another card's file mid-edit. Not touched; waiting, then running again.
- 2026-11-03: rerun passes, 8 of 8, with `--nocapture`: 2,083,667 bytes, 7905 child switches, every line whole; log
  offset `+04:00`, equal to `Get-Date -Format zzz` (`+04:00`); roll with an open writer `Ok`, with a reader without
  delete sharing `Err(MoveFileExW, 32)`, with a reader on `wakeshot.2.log` `Err(MoveFileExW, 5)`; no line lost in any
  case. Mix test run 5 more times alone: all pass, 7826 to 7936 switches, 1.6 s each.
- 2026-11-03: clippy blocked by other cards' files mid-edit: x86_64 `process.rs:498` (`chunks_exact_to_as_chunks`),
  then `store.rs:283` and `:547` (E0282, both targets). Nothing reported in `log.rs`. Waiting, then running again.
- 2026-11-03: research entry "Log writer checks" added. Scratch check (`<scratchpad>\p3-log\size-lag.ps1`): a
  directory listing showed the true size of a file still open for append, so the `file_size` comment no longer claims
  a lag. Clippy retry: `store.rs` compiles now; `process.rs` is mid-edit (E0560, E0599, E0609 at 457, 1033, 1331).
  Waiting again: a scratch script retries clippy every 90 s, at most 20 times.
- 2026-11-03: research line added: `Win32_System_Time` is declared now (the "not declared" line in "Power track plan
  checks" is out of date). Result drafted.
- 2026-11-03: the scratch retry script stopped (by its task id); one foreground retry later `process.rs` compiled
  clean. Final runs: `rustfmt --edition 2024 --check` on `log.rs` exit 0; clippy `-p wakeshot-windows` with and
  without `--all-targets`, `-D warnings`, x86_64 and aarch64: all four exit 0; `cargo test --locked -p
  wakeshot-windows log::`: 8 passed, 1.6 s, 7894 switches. No `wakeshot-log-*` test folder left in `%TEMP%`. Build
  folder `<scratchpad>\p3-log` (target and two scratch scripts) deleted. Card done.

## Result

What was done: `log.rs` filled in. `LogWriter::open`, `LogWriter::roll` and `Log for LogWriter` work; `MemoryLog`,
`Proc`, `Level` and `Log` are unchanged. The only signature change is a parameter name (`path` to `logs_dir`), which
callers never see. `LogWriter` now holds a private `OwnedHandle` and its `Proc` in place of `_private: ()`.

Line: `2026-11-03T18:55:12.345+04:00 [1234.567] tray    INFO  power: text` plus CRLF. Wall time comes from
`clock::now()` (`GetSystemTimePreciseAsFileTime`) and `FileTimeToLocalFileTime` for every line; the offset is local
minus UTC from that same conversion; monotonic seconds come from `clock::now().mono_ms`.

Design choices (mine, as principal designer):

| Choice | Reason | Change it when |
|---|---|---|
| RFC 3339 local time with offset, `[s.mmm]` monotonic, padded process and level, `area: text`, CRLF | Sorts and parses; a reader sees the local clock and the exact instant; brackets mark the boot clock as in `dmesg`; CRLF shows right in any Windows editor "Open log" may pick | A tool must parse the log, or a reader asks for another form |
| Every control character and U+2028, U+2029 become spaces, not only CR and LF | Some editors break lines at NEL, VT, FF or U+2028, and ESC can hide text in a terminal | A caller needs tabs kept |
| Raw `CreateFileW` with `FILE_APPEND_DATA` only and `WriteFile`, not std's append mode | The no-mix result rests on that exact access mask, which std does not document | std documents its mask |
| A writer keeps its handle; after a roll a running process writes on into `wakeshot.1.log` | One open per process; no line lost | Lines of a long-lived process in the rolled file confuse a report |
| Roll at over 1 MiB; `.1` to `.2` first, then `.log` to `.1`, `MOVEFILE_REPLACE_EXISTING`; size read from an open handle | A failure at either step leaves every line in one of the three files | |
| A line over 4 GiB is skipped | The `windows` `WriteFile` wrapper panics there, and a panic aborts the process | Never, in practice |
| `WinError.code` is the Win32 code when the HRESULT wraps one; `u32::MAX` for a std error without an OS code | Codes read like Microsoft's tables (5, 32) | The contracts set another rule |

Files changed: `crates/wakeshot-windows/src/log.rs`; `docs/research.md` (new section "Log writer checks", additions
only); this card.

Checks:

- `rustfmt --edition 2024 --check crates\wakeshot-windows\src\log.rs`: exit 0.
- `cargo test --locked -p wakeshot-windows log::`: 8 passed (`line_format`, `offset_comes_from_the_two_file_times`,
  `local_offset_matches_windows`, `open_creates_the_folder_and_appends`, `failed_write_is_dropped`,
  `appends_from_many_processes_never_mix`, `roll_keeps_three_files`, `roll_with_file_open_elsewhere`), 1.6 s. The mix
  test passed 6 runs: 2,083,667 bytes, 7826 to 7936 switches between the 4 children, every line whole and in order.
- `local_offset_matches_windows`: `+04:00`, the same as `Get-Date -Format zzz` by hand (`+04:00`).
- `cargo clippy --locked -p wakeshot-windows --all-targets --target <t> -- -D warnings` and the same without
  `--all-targets`, for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`: all four exit 0 (final run, after
  other cards' files stopped failing mid-edit; see the Log).

Open items:

- Roll fails while an app holds `wakeshot.2.log` open (5) or holds `wakeshot.log` without delete sharing (32). The
  tray gets `Err`, the log grows until a later start. Change it when a report shows a log that never rolls (delete
  `.2` first, or skip a step that cannot run).
- A daylight saving switch was not tested (Yerevan has none). If a test abroad shows a stale offset right after a
  switch, move to `SystemTimeToTzSpecificLocalTimeEx` with the dynamic zone; each line stays a correct instant anyway.
- Network shares, FAT and ReFS were not tested for the no-mix result.
