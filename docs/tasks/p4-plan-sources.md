# Plan the Sources track (Phase 4)

Phase: 4. Status: done. Needs Haik: yes, at the end: he signs off all five track plans at once (the orchestrator
relays one summary). Agent: `wakeshot-worker`. Size: about 120K tokens.
Runs after: `p1-design-verify-fixes` and the final design check (`docs/reviews/p1-design-final-check.md`), and
`p1-contracts` (the module layout and types in `crates/`). Parallel with: the other four track plan cards
(`p2-plan-core`, `p3-plan-windows-power`, `p4-plan-sources`, `p5-plan-tray-ux`, `p6-plan-ship`; each owns only its
own phase file and card prefix) and the Phase 1 review (read only).

## Goal

Phases 2 to 6 run as parallel tracks (`docs/plan.md`, the note under the Phases table). Plan this track: Claude Code (with timers), Codex and Copilot CLI, `wakeshot run` and hold files, Keep awake, the CLI exe.
Split it into task cards that can be built at the same time as the other four tracks with zero conflicts and no
lost work, test-first, against the contracts in `crates/`.

## Notes from the orchestrator (16:38)

- The shared `wakeshot-windows` modules `store`, `log` and `process` belong to the Windows power track (Phase 3),
  which builds them first. Do not own them; use their fakes until those cards are done, and say which of your cards
  need the real ones.
- `docs/architecture.md` is getting 13 type corrections from `p1-contracts` while you plan. Where it disagrees with
  `crates/`, the code is right.

## Read first

- `CLAUDE.md`; `docs/plan.md`: "How we work" (the parallel and Git rules) and the note under the Phases table.
- `docs/architecture.md`: section 4 ("CLI"), section 6 (hold files, config), section 7 (sources); also `docs/research/claude-code-signals.md` (rules) and `docs/research/other-agent-signals.md`. Skim the rest for anything your track owns.
- `docs/product.md`: the lines your track serves.
- `docs/research.md`: only the sections the design cites for your track.
- `crates/`: the module layout and contract types written by `p1-contracts` (its card Result says what is where).
- `docs/phases/01-design-skeleton.md`: "Values I set" and the answered questions. Questions 10 to 12 are open and
  Haik's: plan around both answers or mark the card that depends on them; do not decide them.

## Deliverables

1. `docs/phases/04-sources.md`: goal, short track notes (only what `architecture.md` leaves open for this track),
   the card table (Card, Needs Haik, Runs after, Status), the order inside the track, and a work log section.
   `architecture.md` stays the design: any design change you need goes in your card's Result as a proposal, with the
   section and the new text.
2. Task cards `docs/tasks/p4-*.md` from `docs/tasks/_template.md`. Each card:
   - fits about 150K tokens of one worker; split bigger work;
   - names the files and folders it owns. Inside `crates/`, own whole module files. Never `lib.rs` or `main.rs`
     module lists, `Cargo.toml` or `Cargo.lock`: those change only through the contracts card the orchestrator runs
     alone, so list what you need under deliverable 3;
   - is test-first: the acceptance checks name the tests and the cargo commands (`CLAUDE.md` "Checks");
   - says its build folder: `CARGO_TARGET_DIR=<scratchpad>\<card name>\target`;
   - is marked when it touches the real laptop: power settings, screen, sound, installs, or needs Haik's hands.
     The orchestrator runs those one at a time across all tracks;
   - fills "Runs after" (inside this track, or another track's card by name) and "Parallel with".
3. In your card's Result: the contract changes this track needs (missing or wrong types, traits, module files,
   dependencies), each with the exact change, for one contracts card the orchestrator runs before the builds.
4. In your card's Result: what this track needs from other tracks and from when, and the fake it uses until then.

## Acceptance checks

- Every item of `architecture.md` and `product.md` this track owns is in a card, or named as integration (later).
- No two cards of this track own one file at the same time, and none owns a file another track's section names.
- Every card that needs the real laptop or Haik is marked as such.
- No code written; `crates/`, `architecture.md`, `product.md` and other tracks' files unchanged.

## Out of scope

- Code; `crates/`; `Cargo.toml`; `docs/architecture.md`; `docs/product.md`; other tracks' phase files and cards.
- Integration cards (wiring the tracks into the two exes): planned after the tracks.
- Git.

## Questions for Haik

Each with the worker's default. Also in `docs/phases/04-sources.md`. Neither blocks a card.

1. **Codex waiting for approval looks like working.** Codex writes no approval prompt to disk, so a turn that waits
   for you holds with no time limit while `codex.exe` lives. Default: hold, no limit (decision 12; the battery floor
   still protects). Other option: a 1 hour limit on a Codex turn with no new rollout line (would also end long silent
   builds).
2. **Copilot waiting for approval.** Copilot writes `permission.requested`. Default: no hold, as Claude waiting without
   Remote Control. Other option: hold up to 1 hour.

## Found outside scope

Problems seen but not fixed.

1. `SessionKey.id`'s doc says Claude's id is the session id, but `sessionId` changes at `/clear` (live test 9); the
   key must be the pid. Fixed through contract change 1e below.
2. Section 7's `WantedFile` cannot find a transcript whose folder is not known (rule 5: glob, never rebuild the name)
   and cannot see a file without opening it (Codex lock files must not be opened). Contract changes 1b and 1c.
3. Section 7 asks for one retry 200 ms later before `Unreadable`, but nothing tells the watch thread which file to
   retry. Contract change 1d (`Fed::RetryOnce`).
4. `p3-process` plans `start_of(&std::process::Child)` only; the hold pid-only rule needs the start time of any pid,
   and the Codex and Copilot fallback needs image names from the process list. Contract change 8.
5. `p3-store` names atomic-write temp files "with the pid and a counter" but not their extension; in `holds\` a temp
   name ending in `.json` would be read as a hold. Cross-track need below.
6. `crates/wakeshot-core/src/lib.rs` shows a write time of 16:46, after this card started; its content is unchanged
   from the `p1-contracts` version I read. Not touched by this card.

## Log

- 2026-11-03 16:27: card written by the orchestrator (Phases 2 to 6 as parallel tracks, Haik's ask).
- 2026-11-03 16:39: worker started; reading the design, research reports and `crates/`.
- 2026-11-03 16:48: read architecture 2 to 7, product, both research reports, `crates/` (core and windows), the
  `p1-contracts` Result, the Phase 1 values and questions. Checked: Claude cron syntax (docs), `croner` and `jiff`
  features (`cargo info`), `windows` 0.62.2 features for Restart Manager and time zones (registry source). Plan:
  write our own cron subset and RFC 3339 code, no new crate.
- 2026-11-03 16:51: facts appended to `docs/research.md` ("Sources track plan checks"). Split decided: 19 cards
  (parsers in the core, five watch cards, four CLI cards, three live cards). Writing the phase file next.
- 2026-11-03 16:53: orchestrator note: a contracts change card (run alone) splits shared files by owner and declares
  every module; plan only whole files that exist after that split. Read `docs/reviews/p1-contracts-domain.md` (B1, M1,
  M2, m1 to m4). Changes: the CLI `main.rs` dispatch and a `command.rs` enum go to the contracts card, one file per
  command for my cards; the watch thread posts through the review's `EventSink`; `format_expires` and
  `parse_expires` signatures are declared by the contracts card with this track as owner.
- 2026-11-03 17:00: power track's phase file appeared; aligned on its names (`p3-store`, `p3-process`, `p3-log`,
  `p3-journal`, `p3-restore`; `process::Liveness`, `FakeLiveness`, `log::Log`, `MemoryLog`, `store::DataDir`). `wakeshot
  run` now uses std `Command` on the resolved path with `raw_arg`, so `p3-process`'s `start_of(&Child)` fits.
- 2026-11-03 17:03: all 19 cards and the phase file written; Result next.
- 2026-11-03 17:13: tray track's phase file uses `shell.rs` from `p4-cli-commands` (path or `ms-settings:` URI);
  card and order updated. `ClaudeSource::new` takes the zone only from `p4-claude-transcript` on (`p4-cron` runs in
  parallel with `p4-claude-status`). No em dashes in the new files. Status done.
- 2026-11-03 18:03 (`p1-track-plan-fixes`): history above is kept as written. `p3-restore`, `p3-apply` and
  `p3-power-props` were dropped at 17:17; read `p3-restore` and `restore_from_journal` as `p3-journal`
  (`power::journal::restore_locked`) plus the core's `p2-ownership-restore`. `PowerApi` and `FakePower` are gone
  (`p1-contracts-split`): `FakePlans` is the one plan fake. The Run value's one writer is `store::AutoStart`
  (cross-track review m12). The cards and phase files carry the current text.

## Result

Done 2026-11-03 17:05. Docs only; no code, no git.

### Files

- New: `docs/phases/04-sources.md` (goal, 17 track notes, card table, ownership table, order, 2 questions, work log).
- New cards (19): `p4-hold-files`, `p4-cron`, `p4-claude-status`, `p4-claude-live-shapes`, `p4-claude-transcript`,
  `p4-codex-source`, `p4-copilot-source`, `p4-watch-folders`, `p4-watch-reads`, `p4-watch-data`, `p4-watch-agents`,
  `p4-watch-thread`, `p4-cli-args`, `p4-cli-run`, `p4-cli-commands`, `p4-cli-restore`, `p4-live-claude`,
  `p4-live-codex`, `p4-live-copilot` (all in `docs/tasks/`).
- `docs/research.md`: new section "Sources track plan checks" (Claude cron syntax from the docs; `croner` and `jiff`
  features; `windows` 0.62.2 Restart Manager and time zone features; std `Command` ignores `PATHEXT`; the Ctrl+C
  ignore flag is inherited).
- This card. Scratch scripts only in the scratchpad.

Size: about 1.7M tokens of worker context over 19 cards (50K to 140K each). Cards that use the real laptop:
`p4-claude-live-shapes`, `p4-live-claude` (Claude Code test sessions, Haiku; no settings); cards that need Haik:
`p4-live-codex`, `p4-live-copilot` (about 20 minutes each, may install the CLI), `p4-live-claude` optional 3 minutes.

### Checks

- Coverage: every item below is in a card or named as integration (table "What each item maps to").
- Ownership: the phase file's table gives each file one card at a time; three files pass from one card to a later one
  (`sources/claude/mod.rs`, the live cards' fixes in done files, `tests/codex_source.rs` and `copilot_source.rs`),
  each after the earlier card is done. No file another track names: checked against `02-core.md` (its
  `policy/hold_files.rs` is the core's hold rules, apart from `holds/file.rs` parsing), `03-windows-power.md` (it lists
  `watch.rs` as not its own), `05-tray-ux.md` (it uses `shell.rs` as this track's, `p4-cli-commands`), `06-ship.md`
  (`p6-readme` runs after `p4-hold-files` and `p4-cli-args`).
- Real laptop or Haik: marked in each card header and in the phase table.
- No code written; `crates/`, `architecture.md`, `product.md` and other tracks' files not changed by this card (I
  wrote only the files above; see "Found outside scope" item 6).

### Contract changes this track needs (for the one contracts card)

Core (`wakeshot-core`):

1. `sources.rs`:
   a. Module lines `pub mod claude; pub mod codex; pub mod copilot; pub mod cron;`. Files: `sources/claude/mod.rs`
      holding only `pub mod status; pub mod transcript;`, and empty `sources/claude/status.rs`, `transcript.rs`,
      `sources/codex.rs`, `sources/copilot.rs`, `sources/cron.rs`, each with one doc line naming Phase 4 as owner.
   b. `WantedFile`, two new variants: `Names { dir: PathBuf, depth: u8, accept: fn(&str) -> bool }` (names only, never
      opened; fed as `Feed::Present` or `Feed::Gone`; depth 0 is files in `dir`, 1 is files in its direct subfolders)
      and `FindTail { root: PathBuf, depth: u8, name_suffix: String }` (the watch thread finds
      `root\<depth folder levels>\*<name_suffix>` and tails it like `Tail`; feeds carry the found path).
   c. `Feed`: new variant `Present`.
   d. `AgentSource::feed` returns `Fed`, new `pub enum Fed { Taken, RetryOnce }`: `RetryOnce` asks the watch thread to
      read the same file again 200 ms later, once (section 7: `Unreadable` only after one retry).
   e. Doc text only: `SessionKey.id`: "Claude: the pid as text, since `sessionId` changes at `/clear`".
      `Liveness::AnyProcess`: "the process that holds `lock_file` open (Restart Manager), else any live process with
      `image` while `lock_file` exists; that fallback can outlive its session". `AgentSource`: "the watch thread owns
      sources as `Box<dyn AgentSource + Send>`".
2. `holds.rs`: module line `pub mod file;`; file `holds/file.rs` with the two signatures the core track compiles
   against (review m3), bodies `todo!()`, owner Phase 4: `pub fn parse_expires(text: &str) -> Option<i64>` and
   `pub fn format_expires(wall_ms: i64) -> String`.
3. `process.rs`: `pub enum Lookup { Running(FileTime), NotRunning, Unknown }` (used by `holds::file::hold_alive` and
   returned by the Windows side's `Liveness::start_of_pid`).
4. `testing.rs`: `FakeSource::feed` returns `Fed::Taken`, except for files listed in a new `pub retry: Vec<FileId>`,
   which get `RetryOnce` once; its test updated.
5. If the contracts card adds `files::read::<T>` (review m2): take the first JSON value and ignore bytes after it, so
   hold files written by scripts read the same way as Claude's files.

Windows (`wakeshot-windows`):

6. `lib.rs` module lines, each with an empty file and a one-line owner doc: `pub mod watch;` (`watch/mod.rs` holding
   only `pub mod dir; pub mod read; pub mod data; pub mod agents; pub mod thread;`, and the five empty files),
   `pub mod lockowner;`, `pub mod localtime;`, `pub mod child;`, `pub mod shell;`.
7. `Cargo.toml` features: `Win32_System_RestartManager`, `Win32_System_Time` (both exist in 0.62.2, research).
8. On the power track's `process::Liveness` trait (its stub comes from the contracts card), two more methods that
   `FakeLiveness` can script: `fn start_of_pid(&self, pid: u32) -> Lookup` (the hold pid-only rule) and
   `fn list(&self) -> Result<Vec<ProcessEntry>, WinError>` with `pub struct ProcessEntry { pub pid: u32, pub start:
   FileTime, pub image: String }` (image file name only; the Codex and Copilot fallback and the lock holder's image).
   `p3-process` fills the bodies from the system process list it already ports.
9. The review's `EventSink` and `FakeEventSink` (M1): the watch thread posts through it.

CLI (`wakeshot-cli`), so no card of any track edits `main.rs` again:

10. `main.rs`, written once, then frozen:

    ```rust
    mod args; mod automatic; mod command; mod keep_awake; mod open_log; mod restore; mod run; mod status;

    use std::process::ExitCode;
    use command::Command;

    fn main() -> ExitCode {
        let argv: Vec<_> = std::env::args_os().skip(1).collect();
        let (global, command) = args::parse(&argv);
        match command {
            Command::Run => run::main(&global),
            Command::KeepAwake { choice, screen } => keep_awake::main(&global, choice, screen),
            Command::Automatic(on) => automatic::main(&global, on),
            Command::Status => status::main(&global),
            Command::Restore => restore::main(&global),
            Command::Log => open_log::main(&global),
            Command::Help => args::help(),
            Command::Bad(why) => args::bad(&why),
        }
    }
    ```

11. `command.rs`, frozen: `pub struct Global { pub dry_run: bool }`; `pub enum Command { Run, KeepAwake { choice:
    wakeshot_core::view::KeepAwake, screen: bool }, Automatic(bool), Status, Restore, Log, Help, Bad(String) }`.
12. Stubs: `args.rs` with today's `reply` logic moved in (`parse`: none or `--help` gives `Help`, anything else `Bad`;
    `help()`, `bad()`; its test), and `run.rs`, `keep_awake.rs`, `automatic.rs`, `status.rs`, `restore.rs`,
    `open_log.rs`, each `pub fn main(..) -> ExitCode` printing "not built yet" to stderr with exit 2. No CLI
    `Cargo.toml` change: the CLI calls Windows only through `wakeshot-windows` (review m1).

### Needs from other tracks

| From | What | Needed by, from when | Fake until then |
|---|---|---|---|
| Contracts card | items 1 to 12 above | every card of this track, before it starts | none |
| `p3-store` | `DataDir` paths (contracts), `write_atomic`, `remove`; temp names that do not end in `.json` (for example `<name>.<pid>.<n>.tmp`) | `p4-cli-run`, `p4-cli-commands`, `p4-cli-restore` | watch cards need only `DataDir` paths on a temp folder |
| `p3-process` | `Liveness` (with items 3 and 8), `ProcessHandle`, `ExitWait`, `start_of(&Child)` | CLI cards and `p4-live-claude` need the real one | `FakeLiveness` in every watch card |
| `p3-log` | `Log`, `LogWriter` (process name `cli` or `tray`) | CLI cards | `MemoryLog` in the watch cards |
| `p3-journal`, `p3-restore` | journal mutex, `restore_from_journal` (review M2 signature) | `p4-cli-restore` | `FakePower` and the safety-net fake in its tests |
| Power and core tracks | `FinishRestoreRequest` (write `state.json`, then delete `restore-request.json`; power plan note 11, integration) | end to end of `wakeshot restore` | `p4-cli-restore` tests a fake tray that deletes the file |
| Core track | uses `format_expires` for Keep awake from the menu; reads timers as due times with jitter and no margin; Claude sessions keyed by pid; `shell` maps to `Working`, `dialog open` to `Idle`; Copilot waiting has `remote: false` | its Keep awake card runs after `p4-hold-files` | signatures from contract item 2 |
| Tray track (gives) | nothing this track needs. (Takes) `shell::open_with_default_app` from `p4-cli-commands`, for a path or an `ms-settings:` URI (`05-tray-ux.md` lines 33, 123, 144); its `p5-haik-check` runs after `p4-cli-commands` | `p4-cli-commands` early | none |
| Ship track | README: links `docs/hold-files.md`, says which agents are tested live (after `p4-live-codex`, `p4-live-copilot`), `winget upgrade` kills `wakeshot run`; no rival credit line planned | `p6-readme` (already after `p4-hold-files`, `p4-cli-args`) | none |

Integration (planned after the tracks): start `WatchThread` in the tray with the three sources, the real
`process::Liveness`, `SystemZone` and the real `EventSink`, after `Started`; `DeleteHold` by the IO thread; end-to-end
checks of `run`, `keep-awake`, `status`, `restore` against a running tray.

### Design changes proposed for `architecture.md` (for the orchestrator to route; not edited here)

1. Section 2, "Boundary traits", the `AgentSource` block: `fn wanted(&self) -> Vec<WantedFile>; // Folder | Names |
   Tail | FindTail` and `fn feed(&mut self, file: &FileId, data: Feed) -> Fed; // Feed: Whole | Appended | Present |
   Gone; Fed: Taken | RetryOnce`.
2. Section 2, `wakeshot-windows` modules table: `watch` becomes "`dir` (folder notices), `read` (whole reads, tails,
   finds, names), `data` (holds, config, restore request), `agents` (sources and their liveness), `thread`"; new rows
   `lockowner` | "which process holds a file open (Restart Manager)"; `localtime` | "`SystemZone`: the PC's zone with
   daylight saving, for cron times"; `child` | "`wakeshot run`: find the program, start it, Ctrl+C handler"; `shell` |
   "open a file with the default app".
3. Section 7, after the first bullet list, add: "`WantedFile::Names` lists files without opening them (Codex and
   Copilot lock files). `FindTail` tails a file whose folder is not known (Claude transcripts in `projects\*\`, Codex
   rollouts in `sessions\YYYY\MM\DD\`). `feed` answers `RetryOnce` for a file that did not parse; the watch thread
   reads it again 200 ms later, once. Claude sessions are keyed by pid, since `sessionId` changes at `/clear`. Timer
   due times include Claude's jitter (a recurring job runs up to 30 minutes late). Cron next times are computed in
   `sources::cron` (own code: Claude's syntax is `*`, numbers, steps, ranges and lists) in local time, through a
   `LocalOffset` the source gets when it is built."
4. Section 7, Codex paragraph: replace from "Phase 4 maps the lock file" to the end of that sentence with "The watch
   thread asks Restart Manager (`RmGetList`) which process holds the lock file open and checks that pid and start
   time; Copilot's in-use lock file is handled the same way. Only if the live test shows Restart Manager cannot see
   the holder does the weaker rule stay, marked 'can outlive its session'."
5. Section 4, "CLI", add: "`run` finds the program in `PATH` with `PATHEXT` (not the current folder), passes the
   arguments as typed (`.bat` and `.cmd` through `cmd.exe /d /s /c`), and ignores Ctrl+C with a handler routine (the
   NULL form is inherited by the child). `--dry-run` before any command uses the dry-run data folder."
6. Section 11, the cron row: "cron next time | none | core | own code in `sources::cron`; `croner` needs `chrono` or
   `jiff`, and `jiff`'s defaults bring system time zone code and a zone database (research, 'Sources track plan
   checks')".

### What each item maps to

| Item | Card |
|---|---|
| Claude status files, `busy` (also with background subagents), `shell`, `waiting` with Remote Control, unknown status, unreadable file (decisions 4, 5, 7, 37) | `p4-claude-status` |
| Claude timers within 1 hour, no hold for the loop fallback, background tasks of non-terminal sessions, `/clear`, resume (decisions 6, 8; rules 3, 5, 6) | `p4-claude-live-shapes`, `p4-claude-transcript`, `p4-cron` |
| Codex CLI and Copilot CLI in v1, live test each, "not tested live" marks (decision 9) | `p4-codex-source`, `p4-copilot-source`, `p4-live-codex`, `p4-live-copilot` |
| Every signal checked against a fact; Codex rule tied to its session (decision 12) | `p4-watch-reads` (Restart Manager), `p4-watch-agents`, `p4-watch-data`, `p4-watch-thread` (exit waits), `p4-live-codex` |
| `wakeshot run` and the documented hold file (decision 10; section 6) | `p4-cli-run`, `p4-hold-files` |
| Keep awake from the CLI; its hold file with the tray's pid (decisions 11, 32) | `p4-cli-commands`, `p4-hold-files`; menu side: core and tray tracks |
| CLI commands, `keep-awake` needs the tray, `run` warns (decision 30, product UX) | `p4-cli-args`, `p4-cli-run`, `p4-cli-commands`, `p4-cli-restore` |
| Watch thread (section 4): folder watch, reads, parsers, exit waits, config, holds, restore request | `p4-watch-folders`, `p4-watch-reads`, `p4-watch-data`, `p4-watch-agents`, `p4-watch-thread` |
| Section 2 "Ported from the spikes": `names.rs`, `first_object`, `find_transcript`, `tail.rs`, `summary.rs` | `p4-claude-status`, `p4-claude-transcript`, `p4-watch-reads` |
| Section 8: format problems logged once a minute per source; Liveness Unknown | `p4-watch-agents`, `p4-watch-data` |
| Section 9: parser fixtures (redacted), broken and unknown files | `p4-claude-live-shapes`, `p4-claude-status`, `p4-claude-transcript`, `p4-codex-source`, `p4-copilot-source` |
| Section 11: cron crate | `p4-cron` (own code, track note 1) |
| The "update Wakeshot" notice, timer margin, the waiting hour | core track (this track reports the facts) |
| Watch thread started in the tray, `DeleteHold`, end-to-end CLI with a tray | integration |
| OpenCode, WSL, app rules, plugins | not v1 (product) |

### Open items

- Questions 1 and 2 above (defaults built).
- Phase 1 Question 12 may add one step to `p4-cli-restore` (noted in that card).
- The design changes above and contract items 1 to 12 need the orchestrator.
