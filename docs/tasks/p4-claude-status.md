# Claude Code source: status files

Phase: 4. Status: done. Needs Haik: no. Agent: `wakeshot-worker`. Size: about 90K tokens.
Runs after: the contracts card (it creates `sources/claude/mod.rs`, `status.rs`, `transcript.rs` and changes
`AgentSource::feed` to return `Fed`). Parallel with: every other card of this track except `p4-claude-transcript`,
which takes over `claude/mod.rs` after this card; every card of tracks 2, 3, 5, 6.
Build folder: `CARGO_TARGET_DIR=<scratchpad>\p4-claude-status\target`. Uses the real laptop: no.

## Goal

`ClaudeSource`, an `AgentSource` that reads `~/.claude/sessions/<pid>.json` and reports each session's activity,
liveness and format health, by rules 1 to 4 of `research/claude-code-signals.md`. Transcripts come in the next card.

## Read first

- `CLAUDE.md` "Facts to keep in mind" (status files, `.key` files).
- `docs/phases/04-sources.md`: track notes 3 and 6.
- `docs/architecture.md` section 7.
- `docs/research/claude-code-signals.md`: "Status file fields", "Status values and transitions", "Recommended
  detection rules" 1 to 4, live tests 7, 8, 10, 11, and "Other facts seen live".
- `crates/wakeshot-core/src/sources.rs` (after the contracts card), `testing.rs` (`FakeSource`).
- `spikes/claude-watch/src/names.rs`, `watch.rs` (`first_object`), `fixtures/README.md`, `fixtures/live-tests.log`.

## Deliverables

1. `crates/wakeshot-core/src/sources/claude/status.rs`: the status file parser. First JSON value only (stray bytes
   after it are seen in the wild); fields read: `pid`, `sessionId`, `procStart` (or `procStartFt`), `status`,
   `waitingFor`, `statusUpdatedAt`, `bridgeSessionId`, `entrypoint`, `kind`; nothing else is kept. Mapping:
   `busy`, `shell` give `Working`; `waiting` gives `Waiting { since_wall_ms: statusUpdatedAt, remote:
   bridgeSessionId is a string }`, except `waitingFor: "dialog open"`, which is `Idle`; `idle` and a missing status
   give `Idle`; any other value gives `Working` plus `FormatHealth::UnknownValue("status", value)` (the value cut to
   32 ASCII characters). `pid` in the file must equal the pid in the name.
2. `crates/wakeshot-core/src/sources/claude/mod.rs`: `ClaudeSource::new(home: PathBuf)` (`p4-claude-transcript` adds
   the zone parameter once `sources::cron` exists; `p4-cron` runs at the same time as this card). `wanted()`: `Folder { dir: <home>\.claude\sessions, accept }`
   where `accept` takes `<digits>.json` only (port `status_file_pid` and its tests: `.key` files, `.fleetview-heartbeat`
   and temp names never pass). `feed`: `Whole` parses; a parse failure answers `Fed::RetryOnce` the first time and
   gives `Unreadable("status file")` after the retry; `Gone` drops the session. `sessions()`: one `SessionFacts` per
   status file, `key.id` the pid as text (track note 3), `liveness: Process { pid, start: procStart }`, `label: "Claude
   Code"`, `timers: []`, `open_tasks: 0`. A file with no `procStart` cannot be tied to its process: `Unreadable`.
3. Fixtures: `crates/wakeshot-core/tests/fixtures/claude/status/`, one status file per shape in `live-tests.log`
   ("Where each shape is": before the first status, `idle` with Remote Control, SDK path, each `waitingFor`, Remote
   Control off), plus broken ones (empty, half written, trailing bytes, `pid` mismatch, unknown status, no `procStart`).
   Built from the redacted log; keep its redaction rules (`fixtures/README.md`).
4. Tests first: `crates/wakeshot-core/tests/claude_status.rs` runs every fixture through `ClaudeSource` and checks the
   facts; unit tests in `status.rs` for each mapping rule; a test that `accept` refuses every `.key` name form.

## Acceptance checks

- `rustfmt --edition 2024 --check <owned files>`; `cargo clippy --locked -p wakeshot-core --all-targets --target
  x86_64-pc-windows-msvc -- -D warnings` and the same for `aarch64-pc-windows-msvc`; `cargo test --locked -p
  wakeshot-core`: clean as the check rule below says, new tests named in the Result.
- No fixture holds a real user name, session id, bridge id or prompt (the secret scan patterns in
  `docs/phases/01-design-skeleton.md`, "Secret scan", find nothing new in the fixtures folder).
- The parser keeps no field beyond the list in deliverable 1 (review the struct; say so in the Result).
- Check rule: `docs/plan.md` "How we work" step 2 and "Checks while other cards run" in this track's phase file. A
  failure in a file another card owns goes in the Log: wait and run again; if it is still there when this card's own
  work is done, list it under "Found outside scope". Never edit, format or `--fix` that file. Every cargo command takes
  `--locked`, and clippy runs once more without `--all-targets` (the libs as they ship; `CLAUDE.md` "Checks"). No
  `--workspace` or `--all` run here; the orchestrator runs those when it closes a wave.

## Out of scope

- Transcripts, timers, background tasks of non-terminal sessions (`p4-claude-transcript`). Liveness checks and file
  reads (watch cards). Policy: the waiting hour, Remote Control, timer margins (core track). Any file not listed above.

## Questions for Haik

None.

## Design choices

Each mine (principal designer), with when to change it.

1. The status file fixtures are whole raw files rebuilt from the 2.1.316 log (its `NEW` line plus the `CHANGE`
   lines), in the key order 2.1.317 writes. Raw bytes, so empty, half-written and trailing-byte files are exact.
   Change it when a new Claude Code version changes a shape: add that file from a live capture.
2. Only a parse failure gets the retry. A whole object with a wrong `pid` or no `procStart` is `Unreadable` at once
   (`status file pid`, `status file procStart`): a truncated write is never a whole object, so a re-read cannot
   change it, and a real rewrite brings its own change notice. Change it when a live log shows a whole object that
   a re-read 200 ms later changes.
3. A bad read keeps the last good read. During the retry the snapshot does not change; an unreadable session keeps
   the last good `procStart`, else `FileTime(0)`, which no process has, so it never counts as alive and is listed
   only so "update Wakeshot" shows. Its activity is `Idle` (the core ignores it). Change it when the core gets a
   notice that needs no session.
4. A `waitingFor` Wakeshot does not know still waits, with `FormatHealth::Ok`. `UnknownValue` would make the core
   count the session as working for its whole life; a wait is capped at one hour. Change it when `FormatHealth`
   gets a notice-only variant.
5. `status: null` reads as missing (idle), as serde reads `null` for an optional key and as Claude writes `null` to
   clear `bridgeSessionId`. A status that is not text is unknown: working, named by its JSON type (`<number>`), so no
   file content reaches the log. Change it when Claude writes `null` or another type for a real state.
6. The name filter takes only `<pid>.json` written the way Claude writes it: lowercase `.json`, ASCII digits, no
   leading zero, pid 1 to `u32::MAX`. So one pid has one name, and pid 0 (the System Idle Process, never ends) never
   passes. `procStart` 0 is refused too. Change it when Claude writes other names.
7. `sessionId` is kept only when it is a plain id (1 to 64 ASCII letters, digits, `-`, `_`), since
   `p4-claude-transcript` puts it into a file search. Another form gives `None`, with no format problem here. Change
   it when Claude's ids take another form.
8. `entrypoint`: `Cli` for `cli`, `Other` for any other text, `None` when missing; `kind`: the four known values plus
   `Other`. No fact of this card depends on them. Change it when a rule needs one client told apart.
9. A leading byte order mark is skipped and a float `statusUpdatedAt` is cut to whole ms, as `files::read` and the
   transcript times do. Claude writes neither today.
10. The source drops a feed for any file it did not ask for (another folder, a nested path, a `.key` or a leading
    zero name) without parsing it, so even a wrong feed cannot turn `.key` bytes into a session.

## Found outside scope

Problems seen but not fixed.

- Track note 6 and the core disagree. Note 6 reports a transcript line of a known kind with a missing field as
  `UnknownValue("transcript", kind)` and says "the status file signal for that session stays". But the core
  (`policy::sessions::steady`) treats any `UnknownValue` as working while the process lives and ignores the
  activity. So after a Claude Code update that drops one transcript field, every idle Claude session would hold the
  PC awake for its whole life. For `p4-claude-transcript` and the Claude parsing review: a notice-only
  `FormatHealth` variant, or no `UnknownValue` from transcripts (contract change through the orchestrator).
- The resume rule in `claude-code-signals.md` ("Phase 4 live shapes", "Rule changes") uses the status file's
  `startedAt`, which this card's field list does not keep. `procStart` serves better: it is the process creation,
  0.4 s to 21 s before `startedAt` in the live fixtures (`docs/research.md`, "Claude status file checks"), so no
  line older than it can come from this process. `StatusFile::proc_start` already has it.
- Claude Code's docs (`code.claude.com/docs/en/env-vars`, read 2026-11-03): `CLAUDE_CONFIG_DIR` overrides the
  configuration folder (default `~/.claude`), "All settings, session history, and plugins are stored under this
  path", suggested for running two accounts side by side. `ClaudeSource::new(home)` reads only
  `<home>\.claude\sessions`, so a user who sets it (or runs a second account) would get no Claude signal. That the
  `sessions\` folder follows it: not verified. For `p4-watch-thread` or `p4-live-claude`; maybe a product question
  (one folder or several).

## Log

- 2026-11-03 16:53: card written by `p4-plan-sources`.
- 2026-11-03 21:14: started by `wakeshot-worker`. Status: running. Reading the card's sources.
- 2026-11-03 about 21:20: read the card's sources, the Results of `p4-claude-live-shapes` and `p4-hold-files`, the
  core's `policy::sessions` (how it uses `FormatHealth`). Plan: `status::parse(name_pid, bytes)` into a `StatusFile`
  that keeps only the listed fields (waitingFor, statusUpdatedAt and bridgeSessionId folded into `activity`);
  `ClaudeSource` keeps per pid the last good read, a retry flag and an Unreadable text. Fixtures as raw status
  files rebuilt from `live-tests.log` lines (2.1.316 values, key order as the 2.1.317 live fixtures show).
- 2026-11-03 21:27: fixtures (14 files and a README) written by a scratch script; `tests/claude_status.rs`,
  `status.rs`, `mod.rs` written. First lib test build failed in my own `status.rs` (a `prop_assert!` message with
  `{name}` captures), fixed within a minute. `--test claude_status`: 10 passed; `--lib sources::claude`: 24 passed.
- 2026-11-03 21:29: clippy (both targets, with and without `--all-targets`) clean after one fix in my test file;
  whole crate test run 390 + 10 passed. Secret scan on my files: two hits in my own unit tests
  (`"bridgeSessionId":"session_x"`), changed to `BRIDGE1`; rescan clean. A one-off probe in `status.rs` (added and
  removed by script) to check that serde fills a struct from a JSON array: the lib test build now fails in another
  card's file, `scenarios/lid.rs:1445` (`Config` without `version`, `p2-core-lid` mid-edit). Waiting, then retrying.
- 2026-11-03 21:31: probe run instead as a temporary test in my own `tests/claude_status.rs` (removed by the same
  script, file checked equal after): serde fills a derived struct from a full-length JSON array, so the `{` check
  stays, now with a test case. Facts in `docs/research.md` "Claude status file checks". The lib test build compiles
  again, but 44 tests in `scenarios::lid` fail (`p2-core-lid`, still running); my 24 + 10 pass. Waiting, then
  retrying the whole crate.
- 2026-11-03 21:34: `docs/research.md` section written (live writes, serde arrays, `procStart` before `startedAt`,
  `CLAUDE_CONFIG_DIR` from the docs). Result drafted. Clippy on all four runs stops at another card's file,
  `state.rs:89` ("multiple fields are never read", `p2-core-lid`); no lint in my files. A scratch script retries
  every 90 s. Final secret scan: 0 hits. Scratch helpers deleted (scan and retry scripts and the target stay for now).
- 2026-11-03 21:37: retry round 3 clean (round 2 had failed in `core/settings.rs`, also `p2-core-lid`). Final runs:
  rustfmt check clean, four clippy runs exit 0, whole crate 445 + 10 tests pass. Nothing left under "Found outside
  scope" from those passing failures. Scratch folder with its target deleted. Status: done.

## Result

Status: done. No contract change needed.

What was done:

- `sources/claude/status.rs`: `status_file_pid(name) -> Option<u32>` and `is_status_file_name(name)` (the `accept`
  filter, ported from the spike, stricter: no leading zero, no pid 0); `parse(name_pid, bytes) -> Result<StatusFile,
  Problem>`, first JSON value only, which must be an object. `StatusFile { pid, session_id: Option<String>,
  proc_start: FileTime, activity: Activity, unknown_status: Option<String>, entrypoint: Option<Entrypoint>, kind:
  Option<Kind> }`; `Entrypoint { Cli, Other }`; `Kind { Interactive, Background, Daemon, DaemonWorker, Other }`;
  `Problem { NotParsed, Pid, ProcStart }` with `what()`, the `Unreadable` text. Mapping as deliverable 1;
  `UNKNOWN_VALUE_CHARS` = 32.
- `sources/claude/mod.rs`: `ClaudeSource::new(home: PathBuf)`, `LABEL` = "Claude Code", `AgentSource` impl as
  deliverable 2. State per pid in a `BTreeMap<u32, Slot>` (snapshot sorted by pid number); `Slot { last:
  Option<StatusFile>, unreadable: Option<&'static str>, retry_pending: bool }`.
- Fixtures: `tests/fixtures/claude/status/`, 14 files and a `README.md` (shape, log lines, redaction).
- Review of the struct (acceptance check): `StatusFile` keeps only `pid`, `sessionId`, `procStart`/`procStartFt`,
  `entrypoint`, `kind`, and `activity` plus `unknown_status` made from `status`, `waitingFor`, `statusUpdatedAt` and
  `bridgeSessionId`; nothing else. The private `Raw` deserializes only those ten keys; serde skips the rest.
  `the_status_file_keeps_only_the_listed_fields` destructures `StatusFile` in full, so a new field breaks it.

For `p4-claude-transcript` (it takes over `mod.rs` and `tests/claude_status.rs`):

- `feed` returns `Taken` early for any file that is not `<sessions dir>\<pid>.json` (`status_pid`); add the
  transcript branch before that return. `Slot::facts` builds each session with `timers: Vec::new()` and
  `open_tasks: 0`: fill them there.
- `Slot::last` is the last good read and survives bad reads (dropped at `Gone`): take `session_id` (already checked
  safe for a file name), `entrypoint` (`Cli` or not, rule 3) and `proc_start` (the resume divider, see "Found
  outside scope") from it.
- In `tests/claude_status.rs` only `fn source()` calls `ClaudeSource::new`, so the `new(home, zone)` change is one
  line.

Files changed: `crates/wakeshot-core/src/sources/claude/status.rs`, `crates/wakeshot-core/src/sources/claude/mod.rs`,
`crates/wakeshot-core/tests/claude_status.rs` (new), `crates/wakeshot-core/tests/fixtures/claude/status/` (new),
`docs/research.md` (new section "Claude status file checks", additions only), this card.

Tests: 34 new, all pass.

- `status.rs` (24): `busy_and_shell_are_working`, `idle_and_no_status_are_idle`,
  `waiting_carries_its_time_and_remote_control`, `waiting_without_a_time_leaves_it_to_the_core`,
  `dialog_open_is_idle`, `every_other_waiting_reason_waits`, `an_unknown_status_is_working_and_named`,
  `an_unknown_status_is_cut_to_32_ascii_characters`, `a_status_that_is_not_text_is_working_and_named_by_its_type`,
  `pid_must_be_the_pid_in_the_name`, `proc_start_ties_the_file_to_its_process`, `only_the_first_json_value_counts`,
  `what_is_not_one_whole_object_is_not_parsed`, `session_id_is_kept_only_as_a_plain_id`, `entrypoint_and_kind`,
  `the_status_file_keeps_only_the_listed_fields`, `accepts_only_pid_json`, `refuses_every_key_name_form`; proptests
  `no_key_name_passes`, `a_name_that_passes_is_the_pid_written_out`, `every_pid_name_passes`,
  `any_bytes_read_without_a_panic`, `facts_are_always_tied_to_the_named_process`,
  `any_status_text_is_known_or_short_ascii`.
- `tests/claude_status.rs` (10): `every_fixture_gives_its_facts`, `every_fixture_has_a_case`,
  `every_live_status_write_reads` (the 150 writes and 17 deletes of the 2.1.317 live fixtures),
  `wanted_is_the_sessions_folder_without_key_files`, `files_it_did_not_ask_for_are_ignored`,
  `a_good_retry_keeps_the_session`, `two_bad_reads_keep_the_last_known_process`, `gone_drops_the_session`,
  `the_snapshot_order_does_not_depend_on_the_feed_order`,
  `the_source_names_itself_and_can_move_to_the_watch_thread`.

Acceptance checks:

- `rustfmt --edition 2024 --check` on `status.rs` and `tests/claude_status.rs`: clean. `mod.rs` declares
  `transcript.rs`, which is not mine, so it was checked as a copy in the scratch folder with empty child files: no
  diff in `mod.rs`.
- `cargo clippy --locked -p wakeshot-core` with and without `--all-targets`, for `x86_64-pc-windows-msvc` and
  `aarch64-pc-windows-msvc`, `-D warnings`: all four exit 0 (final run 21:37, after `p2-core-lid`'s files compiled
  again).
- `cargo test --locked -p wakeshot-core` (final run 21:37): 445 lib tests and the 10 of `claude_status` pass, 0
  fail.
- Secret scan (both pattern sets of `docs/phases/01-design-skeleton.md`, the path patterns, `haika`, the host name,
  and any UUID that is not a `5e55...` fake) over the fixture folder, the test file, `status.rs` and `mod.rs`: 0 hits
  (after `"session_x"` in two unit tests became `BRIDGE1`).
- The parser keeps no field beyond the list: see the review above.

Open items: none for this card. Not verified: the `CLAUDE_CONFIG_DIR` effect on `sessions\` (see "Found outside
scope"); whether pid 4 reports a start time of 0 (guarded either way).
