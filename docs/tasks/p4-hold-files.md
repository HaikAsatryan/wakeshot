# Parse hold files and write the user doc for them

Phase: 4. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 70K tokens.
Runs after: the contracts card (it creates `holds/file.rs` with the signatures of `parse_expires` and
`format_expires` as stubs, so the core track compiles against them before this card is done). Parallel with: every other card of this track
(`docs/phases/04-sources.md`, "Files each card owns") and every card of tracks 2, 3, 5, 6. The core track's Keep awake
card needs `format_expires` from this card, so start it early.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-hold-files\target`. Uses the real laptop: no.

## Goal

Turn a hold file's bytes and name into `HoldFacts` (all but liveness), parse and write the `expires` time, and give
the pure rule that decides a hold's liveness from a process lookup. Document the hold format for users.

## Read first

- `docs/phases/04-sources.md`: track notes 1 and 10.
- `docs/architecture.md` section 6: "Hold format" and the file table; section 5, "Liveness".
- `crates/wakeshot-core/src/holds.rs`, `files.rs` (`HoldFile`, `VersionProbe`), `process.rs` (`FileTime`, `Alive`).

## Deliverables

All in `crates/wakeshot-core/src/holds/file.rs` (module `holds::file`), pure, no I/O. Parse the file's bytes with
`wakeshot_core::files::read` (the contracts card added it: first JSON value only, bytes after it ignored, a UTF-8 byte
order mark skipped, the version rule; `p1-contracts-split` edit 6), never a second JSON entry point:

1. `kind_of(name: &str) -> Option<HoldKind>`: `keep-awake.json` gives `KeepAwake`, `run-<digits>.json` gives `Run`,
   any other `*.json` gives `Script`; anything not ending in `.json` gives `None` (never read).
2. `parse(name, bytes) -> Parsed`: through the contracts card's shared reader (`files::read`, review m2) if it
   exists, else `VersionProbe` first; a newer `version` gives `Parsed::Newer(v)`; bytes that are not
   a `HoldFile` give `Parsed::Bad(&'static str)` (a fixed reason, never file content); else `Parsed::Ok(Hold)` with the
   file, `expires_wall_ms` and the cut `reason`. `reason` is cut to 60 characters (chars, not bytes); `screen` counts
   only for `keep-awake.json`. Take the first JSON value only, as for Claude's files.
3. `parse_expires(&str) -> Option<i64>`: RFC 3339 with `Z` or a `+hh:mm` / `-hh:mm` offset, optional fraction of a
   second (kept to the millisecond), years 1970 to 9999. Anything else is `None`, and the whole file is `Bad` (track
   note 10).
4. `format_expires(wall_ms: i64) -> String`: UTC, whole seconds, `Z` (`2026-11-03T18:00:00Z`). The core track uses it
   for Keep awake from the menu, the CLI for `keep-awake`. Keep the two signatures the contracts card declared; if
   one must change, stop and say so in the Result (the core track compiles against it).
5. `hold_alive(file: &HoldFile, running: Lookup, written: FileTime) -> Alive`, where `Lookup` is the process lookup
   for `file.pid` (`Running(FileTime)`, `NotRunning`, `Unknown`; the contracts card puts it in the core's
   `process.rs`, since the Windows side's `process::Liveness::start_of_pid` returns it). Rule from section 6: with `procStart`, alive only when the start times match; without it, alive only when the process
   started at or before the file's last write time. Not running: `Gone`. Unknown: `Unknown`.
6. `facts(id: HoldId, hold: &Hold, alive: Alive) -> HoldFacts`.
7. `docs/hold-files.md`: the user doc. Where the folder is, the format with every field, the rules (pid required,
   `procStart` form, the pid-only rule, `reason` cut and never a full command line, `expires` in UTC, `screen` only in
   `keep-awake.json`, a file whose process is gone is deleted, a bad file is ignored and logged, write to a temp name
   not ending in `.json` then rename), one PowerShell and one cmd example that write a valid file. Plain short
   English. The README (Phase 6) links to it.

Tests first, in the module: every name rule; the documented JSON form; minimal file; newer version; not JSON;
trailing bytes after the first value; bad `expires` forms (no zone, `24:00`, month 13, leap day in a non-leap year);
`parse_expires(format_expires(t)) == t` for whole seconds over a range of `t` (a `proptest` property); a 61-character
and a multi-byte `reason`; `screen` ignored outside `keep-awake.json`; each `hold_alive` branch, including start equal
to the write time.

## Acceptance checks

- `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-core --all-targets --target
  x86_64-pc-windows-msvc -- -D warnings` and the same for `aarch64-pc-windows-msvc`; `cargo test --locked -p
  wakeshot-core`: clean as the check rule below says, with the new tests listed in the Result.
- `cargo tree -p wakeshot-core -e normal` lists no new crate (the core stays free of Windows code and new
  dependencies).
- `docs/hold-files.md` exists; both examples were run in a temp folder and the files they write parse with
  `parse` (test or a one-off check, recorded).
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- Reading files, folder watching, process lookups (`p4-watch-data`, power track). Deleting hold files (core decides,
  IO thread does). The CLI (`p4-cli-*`). Any file not listed above.

## Questions for Haik

None.

## Found outside scope

Problems seen but not fixed.

- `docs/architecture.md` section 6 says a pid-only process counts "only if it started before the file's last write
  time"; this card (and the code) say "at or before". Equal times happen with coarse clocks, and the writer must
  count, so the doc line should read "at or before".
- `docs/architecture.md` section 6 still says "Phase 4 writes the RFC 3339 parse and format or picks `jiff`": it is
  our own code now (track note 1). Stale line.
- `files.rs` `HoldFile` doc says "documented for users in Phase 4": the doc is now `docs/hold-files.md`, worth naming.
- Passing failures in another card's files during this run (`p2-policy-power`, half done): from about 20:27 a test
  build failed on `policy/lock.rs:59` (import of a missing `policy::ending::clamped_idle_ms`), and clippy had
  unused-import and unused-variable errors in `policy/lid.rs` and `policy/lock.rs`. Both were gone at the final runs
  (20:40), so nothing is left open.

## Log

- 2026-11-03 16:51: card written by `p4-plan-sources`.
- 2026-11-03: worker started; status in progress. Reading phase notes, architecture sections 5 and 6, core modules.
- 2026-11-03 about 19:45: stopped by Haik before any code.
- 2026-11-03 19:58: fresh start; status running. Owned this round: `holds/file.rs`, `docs/hold-files.md`, this card.
  Read the card, phase notes 1, 10 and 18, architecture sections 5 and 6, `holds.rs`, `files.rs`, `process.rs`.
- 2026-11-03 20:05: oracle values for tests from .NET `DateTimeOffset` (pwsh 7.6.9). Checked that
  `(Get-Process).StartTime.ToFileTimeUtc()` equals the raw `GetProcessTimes` creation time (pwsh 7.6.9: 337 of 337;
  Windows PowerShell 5.1: 332 of 332), so the user doc can give it as the way to fill `procStart`.
- 2026-11-03 20:15: `holds/file.rs` written (kind_of, Hold, Parsed, parse, parse_expires, format_expires, hold_alive,
  facts) with 19 tests; `cargo test --locked -p wakeshot-core holds::file`: 19 passed. Next: the user doc.
- 2026-11-03 20:25: `docs/hold-files.md` written. Both examples run in a temp folder from the doc text (only the
  folder line and the work line swapped): pwsh 7.6.9, Windows PowerShell 5.1 and cmd each wrote a file whose pid is
  the process that ran, and each deleted it at the end. Their bytes are now a test, plus a test that the doc's JSON
  block is the architecture's form.
- 2026-11-03 20:27: test build fails in another card's file: `policy/lock.rs:59` imports
  `policy::ending::clamped_idle_ms`, which does not exist yet (`p2-policy-power`, half done). Waiting, then retrying.
- 2026-11-03 20:35: retry passed: 21 tests in `holds::file`, 246 in the crate. RFC 3339 grammar read; the rename
  steps of both examples checked over an existing file. Facts in `docs/research.md` "Hold file checks".
- 2026-11-03 20:40: all acceptance checks run (below); status done. Scratch folder deleted.

## Result

Status: done. Signatures of `parse_expires` and `format_expires` unchanged.

What was done, all in `crates/wakeshot-core/src/holds/file.rs` (pure, no I/O):

- `kind_of(name)`: `keep-awake.json` gives `KeepAwake`, `run-<ASCII digits>.json` `Run`, any other `*.json`
  `Script`, anything else `None`.
- `Hold { kind, file, expires_wall_ms, reason, screen }` and `Parsed { Ok(Hold), Newer(u32), Bad(&'static str) }`.
- `parse(name, bytes)`: through `files::read` only. `Bad` reasons are fixed texts: the name, not a hold file, pid 0,
  bad `expires`.
- `parse_expires`, `format_expires`, `hold_alive`, `facts` as the card says.
- `docs/hold-files.md`: the user doc, with one PowerShell example (5.1 and 7) and one cmd example.

Design choices (mine), each with when to change it:

1. Name rules ignore ASCII case, as NTFS does, so `KEEP-AWAKE.JSON` is Keep awake. Change it when a folder with
   per-directory case sensitivity shows up in a report.
2. `reason`: control characters become spaces, then trim, then cut to 60 chars; empty gives `None`. A line break
   would split the log line and the tooltip. Change it when the tray wants the raw text.
3. `pid` 0 is `Bad`: the System Idle Process never ends, so the hold never would. No trigger expected.
4. `parse_expires` takes exactly RFC 3339 `date-time`: lowercase `t`/`z` allowed, `-00:00` as UTC, a fraction cut
   (not rounded) to ms, `:60` read as `:59`. A space for `T` is refused. Change it when a common tool writes a form
   we refuse.
5. `format_expires` drops the ms and clamps to 1970 to 9999, so its text always reads back. Change it when a
   Keep awake must end on a fraction of a second.
6. The cmd example gets its own pid as PowerShell's `ParentProcessId`, since cmd has no way to read it. Change it
   when `wakeshot run` covers that case in the doc.

Files changed: `crates/wakeshot-core/src/holds/file.rs`, `docs/hold-files.md` (new), `docs/research.md` (new
section "Hold file checks", additions only), this card.

Tests (21, in `holds::file::tests`): `names_give_kinds`, `reads_the_documented_form`,
`the_user_doc_json_is_the_documented_form`, `files_the_doc_examples_wrote_parse`, `reads_a_minimal_file`,
`a_newer_version_is_newer_whatever_its_fields`, `what_is_not_a_hold_file_is_bad`,
`bytes_after_the_first_value_are_ignored`, `a_bad_expires_makes_the_file_bad`, `expires_forms_that_parse`,
`expires_forms_that_do_not_parse`, `expires_formats_in_utc_whole_seconds`, proptests `whole_seconds_round_trip`,
`any_wall_ms_formats_to_text_that_reads_back`, `an_offset_names_the_same_instant`, `reason_is_cut_to_60_characters`,
`reason_is_one_trimmed_line`, `screen_counts_only_in_keep_awake_json`, `hold_alive_with_proc_start_needs_that_process`,
`hold_alive_without_proc_start_needs_a_start_by_the_last_write`, `facts_carry_what_counts`.

Acceptance checks (final run 2026-11-03 20:35 to 20:40):

- `rustfmt --edition 2024 --check holds/file.rs`: clean (the file has no child modules).
- `cargo clippy --locked -p wakeshot-core` with and without `--all-targets`, for `x86_64-pc-windows-msvc` and
  `aarch64-pc-windows-msvc`, `-D warnings`: all four exit 0.
- `cargo test --locked -p wakeshot-core`: 246 passed, 0 failed (21 of them this card's).
- `cargo tree --locked -p wakeshot-core -e normal`: only `serde` and `serde_json` and their own crates, as before. No
  new crate; no `Cargo` file touched.
- `docs/hold-files.md` exists. Both examples were cut from the doc and run in a scratch folder (pwsh 7.6.9, Windows
  PowerShell 5.1, cmd); each wrote a valid file with the pid of the process that ran it and deleted it at the end.
  The bytes they wrote parse with `parse` (test `files_the_doc_examples_wrote_parse`).

Notes for other cards:

- `p4-watch-data`: call `parse` with the bare name; log `Bad`'s text as is; then
  `hold_alive(&hold.file, liveness.start_of_pid(hold.file.pid), written)` and `facts`.
- `p2-core-holds`: `format_expires` drops the ms. Find Keep awake by `HoldKind::KeepAwake`, not by the `HoldId`
  text, since the name rules ignore case.
- `p6-readme`: link `docs/hold-files.md`.

Open items: none. Not verified: `procStart` from PowerShell for a process that started in the repeated hour of a
daylight saving change; whether `Move-Item -Force` and `move /y` replace in one rename.
