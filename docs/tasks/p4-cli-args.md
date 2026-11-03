# CLI: parse the command line

Phase: 4. Status: ready. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 50K tokens.
Runs after: the contracts card (it writes the CLI's `main.rs` dispatch, the `command.rs` enum and a stub
`args.rs`). Parallel with: every other card of this track and every card of tracks 2, 3, 5, 6.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-cli-args\target`. Uses the real laptop: no.

## Goal

Turn `wakeshot`'s arguments into the `Command` the dispatch in `main.rs` runs, with the help text and plain error
messages; and find the exact text of the command after `run --`, so the child gets its command line as typed.

## Read first

- `docs/product.md` UX: the CLI commands.
- `docs/phases/04-sources.md`: track notes 11 and 13.
- `docs/architecture.md` section 4, "CLI".
- `crates/wakeshot-cli/src/main.rs`, `command.rs`, `args.rs` (after the contracts card).

## Deliverables

`crates/wakeshot-cli/src/args.rs`, plus one line in `command.rs`: once `args::parse` builds every command, remove the
one `#[expect(dead_code, ...)]` attribute there (the only edit any track makes to that contract file;
`p1-contracts-split` edit 4).

`args.rs`:

1. `parse(argv) -> (Global, Command)` for: `run -- <command>`, `keep-awake 30m|1h|2h|4h|on|off [--screen]`,
   `automatic on|off`, `status`, `restore`, `log`, `--help` / `help` / no arguments; global `--dry-run` before the
   command. `30m|1h|2h|4h` map to `KeepAwake::Minutes(30|60|120|240)`, `on` to `UntilOff`, `off` to `Off`;
   `off --screen` is an error. `run` without `--`, or with nothing after it, is an error that shows the right form.
   Anything else gives `Command::Bad` with one plain line.
2. `help()` and `bad(why)`: write to stdout (help, exit 0) or stderr (error, exit 2), ignoring write errors (a closed
   pipe must not panic; the release build aborts on panic). The help lists each command in one line, and `--dry-run` as
   a test aid.
3. `run_tail(raw: &OsStr) -> Option<&OsStr or OsString>`: given the raw command line (`GetCommandLineW`, read by the
   run card), the exact text after the `--` that follows `run`, found with the same splitting rules as the MSVC C
   runtime (quotes, `""`, backslashes before quotes; the program name's own rule), so nothing is re-quoted.

Tests first: every command and every error; `--dry-run` before and after the command (after is an error); `run_tail`
with quoted paths, spaces, `--` inside quotes, `--` as a later argument of the child, backslashes and quotes, tabs,
non-ASCII text, and a program path with spaces. A `proptest` property: for random argument lists quoted the MSVC way,
`run_tail` returns exactly the quoted child part (`proptest` is already a dev-dependency of `wakeshot-cli`, added by
`p1-close-contracts`, so no `Cargo` file changes here).

## Acceptance checks

- `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-cli --all-targets --target
  x86_64-pc-windows-msvc -- -D warnings` and the same for `aarch64-pc-windows-msvc`; `cargo test --locked -p
  wakeshot-cli`: clean as the check rule below says, new tests named in the Result.
- `wakeshot --help` and `wakeshot nonsense` from a built exe: the right text and exit codes (recorded).
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- Running any command (`p4-cli-run`, `p4-cli-commands`, `p4-cli-restore`). `main.rs`, `command.rs` (contracts card;
  if the enum must change, stop and say so in the Result), except the one attribute line above. Any file not listed
  above.

## Questions for Haik

Each with the worker's default.

## Found outside scope

Problems seen but not fixed.

## Log

- 2026-11-03 16:53: card written by `p4-plan-sources`.

## Result

What was done, files changed, checks and outcomes, open items.
