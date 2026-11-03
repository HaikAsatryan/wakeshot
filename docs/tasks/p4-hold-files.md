# Parse hold files and write the user doc for them

Phase: 4. Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 70K tokens.
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

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03 16:51: card written by `p4-plan-sources`.

## Result

What was done, files changed, checks and outcomes, open items.
