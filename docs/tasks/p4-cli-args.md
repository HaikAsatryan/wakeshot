# CLI: parse the command line

Phase: 4. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 50K tokens.
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

None open. Decided by the worker, for Haik's review (each with its reason and a "change it when"):

1. **Words match in any ASCII case** (`STATUS`, `Keep-Awake 1H`, `--DRY-RUN`), as Windows' own commands do. Change it
   when a command needs two words that differ only in case.
2. **Help has four spellings: `--help`, `-h`, `/?` and `help`**, and `--help`, `-h` or `/?` after a command
   (`wakeshot status --help`) shows the help too, except after `run --`, where they belong to the child. Why: these
   are what Windows and Unix users type first, and help beats an error. Change it when one command gets its own help
   text.
3. **`keep-awake` takes `--screen` before or after the length**, and the length only once. Change it when a second
   flag comes.
4. **Errors name the right form** and go out as one line, exit 2 (the contracts card's `bad`): such as `wakeshot:
   'run' needs '--' before the command, such as 'wakeshot run -- npm test'. Run 'wakeshot --help'.` A `--dry-run`
   after the command is an error, not quietly taken, so a test never runs against the real data folder by a typo.
   Change it when a script needs the errors apart by exit code.
5. **`run -- ""`** (an empty program) is an error in parsing: there is nothing to run.

## Found outside scope

None.

## Log

- 2026-11-03 16:53: card written by `p4-plan-sources`.
- 2026-11-04 00:38: started by `wakeshot-worker` (bundle with `p4-cli-run`); read the card, product UX, track notes 11 to 18, architecture sections 4 and 6, the `p4-cli-commands` and `p4-cli-restore` Results.
- 2026-11-04 00:46: `args.rs` written (parse, help, errors, `run_tail`, `split_program`); the dead-code `expect` in
  `command.rs` removed. Rules checked in std 1.99.1's `parse_lp_cmd_line` and Microsoft's "Parsing C command-line
  arguments" (`docs/research.md`, "CLI argument checks").
- 2026-11-04 00:52: rustfmt clean; `cargo test --locked -p wakeshot-cli`: 60 passed, 3 ignored (others' by-hand
  tests), plus 3 in `tests/exe.rs`; clippy clean on 4 runs.
- 2026-11-04 00:54: built-exe checks: `--help` exit 0, `nonsense` exit 2; and the five `--dry-run` checks the two
  command cards left open, through the exe; the dry-run files they made removed.
- 2026-11-04 00:55: done.

## Result

Done 2026-11-04 00:55.

What was built:

- `crates/wakeshot-cli/src/args.rs`:
  - `parse(argv) -> (Global, Command)`: `run -- <command>` (only the shape is checked: `--` next, then a non-empty
    program), `keep-awake 30m|1h|2h|4h|on|off [--screen]` (`Minutes(30|60|120|240)`, `UntilOff`, `Off`),
    `automatic on|off`, `status`, `restore`, `log`, help, and `--dry-run` before the command. Anything else gives
    `Command::Bad` with one plain line (Questions, items 1 to 5).
  - `help()` (stdout, exit 0) and `bad(why)` (stderr, exit 2) as the contracts card left them; write errors ignored.
    The help lists each command in one line, at most 80 characters wide, and `--dry-run` as a test aid.
  - `run_tail(raw) -> Option<OsString>`: the raw text after the `--` that follows `run`, from the child's first
    argument to the end of the line, exactly as typed. `split_program(tail) -> Option<(OsString, OsString)>`: the
    child's program as `args_os` reads it, and the raw rest after it, for `p4-cli-run` (it needs both: the program to
    resolve, the rest to pass with `raw_arg`). Both use a port of std's own Windows splitter, so they find the same
    `run` and `--` that `parse` saw. Until `p4-cli-run` calls them they carry a `cfg_attr(not(test), expect(dead_code))`
    that that card removes.
- `crates/wakeshot-cli/src/command.rs`: the one `#[expect(dead_code, ...)]` removed.
- `crates/wakeshot-cli/tests/exe.rs`: the built exe's help, an unknown command and a late `--dry-run`.

New tests: `args::tests::help_for_no_arguments_and_every_help_form`, `every_command`, `keep_awake_lengths_and_screen`,
`words_match_in_any_case`, `dry_run_goes_before_the_command`, `run_errors_show_the_right_form`, `keep_awake_errors`,
`other_errors`, `help_names_every_command_and_the_test_aid`, `run_tail_is_the_text_after_run_dashes`,
`run_tail_with_a_program_path_with_spaces`, `run_tail_keeps_quoted_paths_and_spaces_as_typed`,
`run_tail_reads_dashes_and_run_through_quotes`, `run_tail_leaves_a_later_dashes_to_the_child`,
`run_tail_with_backslashes_and_quotes`, `run_tail_with_tabs`, `run_tail_keeps_non_ascii_and_lone_surrogates`,
`run_tail_without_the_run_form_is_none`, `microsofts_examples_split_the_documented_way`,
`split_program_gives_the_program_and_the_raw_rest`, and the property `run_tail_returns_exactly_the_quoted_child_part`
(random child arguments from quotes, backslashes, blanks, `--`, Armenian and an emoji, quoted std's way or with `""`,
behind five program name forms, with and without `--dry-run`, `run` and `--` bare or quoted: `run_tail` gives exactly
the quoted child part, it splits back to the arguments, and `parse` agrees). `tests/exe.rs`:
`help_goes_to_stdout_with_exit_0`, `an_unknown_command_is_one_line_on_stderr_with_exit_2`,
`a_late_dry_run_is_an_error_before_anything_runs`.

Checks:

1. `rustfmt --edition 2024 --check` on `args.rs`, `command.rs`, `tests/exe.rs`: clean. `cargo clippy --locked -p
   wakeshot-cli` with and without `--all-targets`, `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`, `-D
   warnings`: clean (4 runs). `cargo test --locked -p wakeshot-cli`: 60 passed, 3 ignored (the by-hand tests of the
   command cards), and 3 passed in `tests/exe.rs`.
2. Built exe (debug): `wakeshot --help` printed the help to stdout, exit 0; `wakeshot nonsense` printed `wakeshot:
   unknown command 'nonsense'. Run 'wakeshot --help'.` to stderr, exit 2 (`docs/research.md`, "CLI argument checks").
3. The open items of `p4-cli-commands` and `p4-cli-restore`, through the exe with `--dry-run`: `log`, `status`,
   `keep-awake 1h`, `automatic off`, `restore` gave the same text and codes as their in-process checks; the dry-run
   files they made were removed (same research section).

Open items: none for this card. `p4-cli-run` removes the two dead-code attributes when it calls `run_tail` and
`split_program`.
