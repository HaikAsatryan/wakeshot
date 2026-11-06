# OpenCode: how to tell it is working

Research for Wakeshot, 2026-11-06, card `docs/tasks/p4-opencode-source.md` Part A. It replaces the "OpenCode" section
of `other-agent-signals.md` (2026-11-03, 2.x only), which stays as it was.

Marks: **verified** = seen in source code, official docs or a probe on this laptop (path or command given).
**not verified** = guessed, read elsewhere, or not checked. No OpenCode was installed or run.

Sources, read as data in the card's scratch folder, never built or run:

- `anomalyco/opencode` tag `v1.19.5` (commit `810c0eb`, 2026-10-28): the npm package `opencode-ai` 1.19.5, built from
  `packages/opencode` (`packages/opencode/script/publish.ts:57`, `name: pkg.name + "-ai"`).
- `anomalyco/opencode` tag `v2.2.5` (commit `9bbb8ce`, 2026-10-30): the npm package `@opencode/cli` 2.2.5
  (`packages/cli`).
- The published npm tarball of `@opencode/cli` 2.2.5, the npm package `xdg-basedir` 5.1.0, the crate
  `libsqlite3-sys` 0.39.1.

Paths below are under `packages/` of the tag named. License MIT (GitHub API `license.spdx_id`).

## Summary

| Version | Working signal | Where | Confidence |
|---|---|---|---|
| 1.x (`opencode-ai`) | The newest `message` row of a session is an assistant message with no `time.completed` | `%USERPROFILE%\.local\share\opencode\opencode.db`, tables `session`, `message` | High for the rule (source read); live behavior not tested |
| 2.x (`@opencode/cli`) | `session_v2.time_busy > time_idle` (or `time_idle` null). Do not test `time_suspended`: it is now set while a turn runs | same file, table `session_v2` | High (source read); live not tested |
| 2.x future work | `schedule` rows with `status = 'active'`: due at `max(next_due_at, retry_at)` | same file, table `schedule` | High (source read) |

Both versions: waiting for a permission prompt is in memory only, so it reads as working. Both run sessions in a
process with the image `opencode.exe` (the desktop app's sidecar is `opencode-cli.exe`). Both keep account tokens in
the same database file (see "Secrets in the database").

Which version people run: npm downloads in the week 2026-10-29 to 11-04 were 4,292,609 for `opencode-ai` and 610,990
for `@opencode/cli` (orchestrator's numbers in the card; not verified by me). The npm `latest` tags were 1.19.5 and
2.2.5 on 2026-11-06 (verified: `registry.npmjs.org/<name>/latest`).

## Facts shared by both versions

- **Folders.** Both use `xdg-basedir` 5.1.0 (`core/package.json`), whose code is `env.XDG_DATA_HOME ||
  path.join(homedir, '.local', 'share')`, the same for state (`.local\state`), config (`.config`) and cache (`.cache`),
  on every OS (verified: its `index.js:7-16`). 1.x: `core/src/global.ts:3-14`. 2.x: `util/src/global-roots.ts`
  (from the old research; not re-read).
- **Database file.** `<data>\opencode.db` for the channels `latest`, `beta`, `prod` (1.x) or `latest`, `dev`, `beta`,
  `next`, `prod` (2.x); else `opencode-<channel>.db`. `OPENCODE_DB` overrides it (absolute, or relative to the data
  folder); `OPENCODE_DISABLE_CHANNEL_DB=1` forces the plain name. Verified: 1.x `core/src/database/database.ts:43-55`,
  2.x `cli/src/database-path.ts:4-13`. So 1.x and 2.x share one file; 2.x copies 1.x rows into its own tables
  (`core/src/database/v1-migration.bun.ts:576-577` counts the 1.x `session` table) and drops no 1.x table (no
  `DROP TABLE` for them in `core/src/database/migration` or `v1-migration.bun.ts`, grep).
- **WAL mode.** On open: `journal_mode = WAL`, `synchronous = NORMAL`, `busy_timeout = 5000`, then
  `wal_checkpoint(PASSIVE)` (verified: 1.x `core/src/database/database.ts:27-32`; 2.x per the old research).
- **No new SQL syntax in the schema.** No `STRICT`, generated columns or `->>` in either migration folder (grep).
  Partial indexes (SQLite 3.8.0) are used. So any current SQLite can parse the schema.
- **Secrets in the database.** Table `account` and `control_account` have `access_token` and `refresh_token`;
  table `credential` has `value` (verified: 1.x `core/src/account/sql.ts:10-11,30-31`, `core/src/credential/sql.ts:9`;
  2.x `core/src/database/schema.gen.ts:16-51`). SQLite stores each table in its own b-tree pages and a query reads
  only the pages of the tables and indexes it touches (SQLite file format; not traced on these files), so a query on
  `session_v2`, `message` or `schedule` never loads those values into the reader's memory. The file as a whole does
  hold tokens.
- **Process image.** Both npm packages' `postinstall.mjs` hard-link (or copy) the native `opencode.exe` from the
  platform package (`opencode-windows-x64`, `@opencode/cli-windows-x64`, and the ARM64 ones) to the package's
  `bin\opencode.exe`, which npm's shim runs. So no `node.exe` sits in between (verified: 1.x
  `opencode/script/postinstall.mjs:28-29,146-153`; 2.x published `postinstall.mjs:26-27,113-124` and
  `package.json` `bin`). The old research's `node.exe` running `bin/opencode.cjs` is the source tree's dev launcher,
  not what npm installs.
- **Waiting for the user.** Pending permission prompts are a map in memory: 1.x `opencode/src/permission/index.ts:99`,
  2.x `core/src/permission.ts:133,232`. Nothing on disk marks them. So waiting reads as working.
- **Prevents sleep itself?** Not re-checked for 1.x; 2.x CLI no (old research).

## 1.x (`opencode-ai` 1.19.5)

### Architecture that matters

- Each `opencode` TUI process runs its own server in a Worker thread of the same process
  (`opencode/src/cli/cmd/tui.ts:249`, `new Worker(...)`). `opencode serve`, `web`, `run` and `acp` also run
  sessions in their own process (`opencode/src/cli/cmd/`). `opencode attach` is a client only (not verified beyond
  the file name). So sessions live inside whichever `opencode.exe` started them, and several processes can write the
  same database.
- No background service and no `service.json` in 1.x (`grep service.json` finds none in `packages/opencode`; not
  re-verified per file).

### Working vs idle

- The busy / idle / retry status is memory only: `opencode/src/session/status.ts:26-48` keeps a `Map` and publishes
  `session.status`, an event with no `durable` option (`schema/src/session-status-event.ts:35-41`). Only durable
  events reach the database (`core/src/event.ts:205-350`: one transaction runs the projectors, then inserts into
  `event_sequence` and `event`).
- What reaches the database: `message.updated` and `message.part.updated` are durable (`schema/src/v1/session.ts`,
  the `events` object with `...options`); the projector upserts the `message` row with `id`, `session_id`,
  `time_created` (= the message's `time.created`) and `data` (the message info as JSON, without `id` and
  `sessionID`) (`core/src/session/projector.ts:259-271`). `message.part.delta` (streamed text) is not durable
  (`schema/src/v1/session.ts:632-633`), so streaming does not write the file.
- `message.data` of an assistant message has `role: "assistant"`, `time.created`, optional `time.completed`,
  `parentID`, optional `finish`, optional `error` (`schema/src/v1/session.ts:453-484`). A user message has
  `role: "user"` and `time.created` (`:332-336`).
- The loop (`opencode/src/session/prompt.ts:1081-1340`): each step writes a new assistant message with no
  `time.completed` (`:1186-1201`), then streams. Tool calls run inside the stream. `time.completed` is set in the
  processor's `cleanup()` after the stream ends (`opencode/src/session/processor.ts:553-611`, run by
  `Effect.ensuring` at `:690`), and on an interrupt by `finalizeInterruptedAssistant` (`prompt.ts:1203-1211`). The
  loop ends when the last assistant message has a `finish` other than `tool-calls` or `unknown`, no tool calls, and
  answers the last user message (`prompt.ts:1111-1130`).
- **Rule.** A session is working when its newest `message` row (by `time_created`, then `id`) is an assistant
  message with no `time.completed`. That covers a running tool (a long `bash`), a retry wait, a compaction (an
  assistant message with `summary`), a shell started with `!` (`prompt.ts:363-396`), and a waiting permission.
  Gaps read as idle for milliseconds: between steps, and from a new prompt to its first assistant row. User input
  just reset the idle timer in the second case.
- Subagents run as child sessions (`session.parent_id`) with their own messages, so the child reports working.
- **Crash.** A killed process leaves its last assistant row without `time.completed`, and 1.x has no startup
  cleanup for it (grep for orphan or cleanup code finds only tool parts, `processor.ts:553-611`). Liveness must
  catch this (below).

### Background and scheduled work

- No `schedule` table in 1.19.5 (the `sqliteTable(` list in `core/src`). Background jobs: not traced; a background
  subagent is a child session and shows as working.

## 2.x (`@opencode/cli` 2.2.5): re-check of the 2026-11-03 research

### Architecture

- Unchanged in substance: sessions run in a service process `opencode.exe serve --service`
  (`cli/src/services/service-config.ts:150`, `[...selfCommand(), "serve", "--service"]`; for the compiled exe
  `selfCommand()` is only `process.execPath`, `cli/src/util/process.ts:5-11`). Without the service a TUI or `acp`
  starts a private server `opencode.exe serve --stdio --port 0` (`cli/src/services/standalone.ts:17-21`). A user can
  also run `opencode serve`. All three are `opencode.exe` with a `serve` argument.
- The desktop app runs `opencode-cli.exe serve --service` on the same default folders
  (`desktop/src/main/service/background-service.ts:46-67`, `desktop-cli.ts:212`). Not researched further.
- `service.json` holds the service password: never read it (`cli/src/services/service-registration.ts:24-40`).
  There is no separate lock file; the registration file itself is the lock (`:33-78`).

### Working vs idle: what changed

- `time_busy` is new: migration `20261029073502_session_time_busy`. It is set to the event time on
  `session.execution.started` (`core/src/session/projector.ts:790-797`).
- `time_idle` and `idle_outcome` are set on `session.execution.succeeded|failed|interrupted`, at least 1 ms after the
  old `time_idle`; an `interrupted` with reason `shutdown` sets neither (`projector.ts:506-535`). Unchanged.
- **`time_suspended` changed meaning.** It is now a write-ahead claim: `SessionStore.claim` sets it in the same
  transaction as `execution.started`, and every terminal event clears it, except a shutdown interrupt, which keeps
  it so the next service start resumes the turn (`core/src/session/execution.ts:73-83`,
  `core/src/session/store.ts:218-237`). At boot the managed service resumes every claimed top-level session, at most
  10 times, then fails it (`core/src/session/execution/restart.ts:34-100,221-259`). So a running session has
  `time_suspended` set, and the old rule's `time_suspended IS NULL` would read every running session as idle.
- **Rule.** A session is working when `time_busy IS NOT NULL AND (time_idle IS NULL OR time_busy > time_idle)`, and
  a live server process started at or before `time_busy` (below). A shutdown-claimed session with no server running
  is not working; when a server starts again it resumes the turn and writes a new `time_busy`.
- Columns read: `id`, `parent_id`, `time_busy`, `time_idle` (`core/src/database/schema.gen.ts:251-290`; new since the
  old research: `fork_session_id`, `fork_boundary`, `time_viewed`, `resume_attempts`).

### Schedules

- Table `schedule` (`schema.gen.ts:136-159`): `status` is `active`, `paused` or `done`
  (`schema/src/schedule.ts:114`). A row fires when `status = 'active'`, `next_due_at <= now` and `retry_at` is null
  or passed (`core/src/schedule.ts:478-486`). So its due time is `max(next_due_at, coalesce(retry_at, next_due_at))`.
  `session_id` can be null. Only a running server fires schedules; a `kv` lease (`LEADER_KEY`) picks one server and is
  renewed only while active rows exist (`schedule.ts:558-605`).

### Background jobs

- Table `job` (`schema.gen.ts:72-85`), `status` `running|completed|error|cancelled` (`core/src/job.ts:35`). A row is
  written when a job is backgrounded and when it ends (`job.ts:224-256,283,415,476`), and deleted once consumed
  (`:497,542`). An explicit cancel writes `cancelled` (`:459-476`), but an exit by interrupt alone does not write
  (`:267-283`), so a `running` row can outlive its job while the same service lives on. Not safe as a hold signal
  without a live test.

## Reading SQLite from Rust, read only, never blocking OpenCode

### Facts

- **Windows' own SQLite.** `C:\Windows\System32\winsqlite3.dll` here is 3.53.4 (file version, PowerShell
  `Get-Item`); the Windows SDK 10.0.26100 has `um\x64\winsqlite3.lib`, `um\arm64\winsqlite3.lib` and
  `winsqlite\winsqlite3.h` (header `SQLITE_VERSION "3.51.1"`). Microsoft Learn, "Use a SQLite database in a Windows
  app" (updated 2026-09-10): "All supported versions of Windows support SQLite, so your app does not have to package
  SQLite libraries. Instead, your app can use the version of SQLite that comes installed with Windows", and "The
  Windows version of SQLite is maintained by Microsoft in coordination with SQLite.org". "Custom SQLite versions"
  (Microsoft.Data.Sqlite docs) names `winsqlite3.dll` "the system SQLite library on Windows 10". On ARM64: the SDK's
  ARM64 import library suggests the DLL ships there too; not checked on an ARM64 machine. The version a given
  Windows 11 build carries is not documented.
- **Probe on this laptop** (`<scratchpad>\p4-opencode-source\scripts\probe.ps1`, P/Invoke into `winsqlite3.dll`, its
  own scratch database, 2026-11-06): `json_extract` and `json_each` work; `pragma compile_options` has
  `THREADSAFE=1` and omits only `LOCALTIME`. A `SQLITE_OPEN_READONLY` connection read committed rows while a writer
  held the database in WAL mode; it read the old snapshot while the writer held an open `BEGIN IMMEDIATE`, with no
  `SQLITE_BUSY`; a write through it failed with code 8 (`SQLITE_READONLY`); the writer's
  `wal_checkpoint(TRUNCATE)` succeeded while the reader had no open statement. **A read-only open of a WAL database
  with no `-wal` and `-shm` created both files and left them after close.**
- **SQLite's rules** (sqlite.org/wal.html): writers do not block readers and readers do not block writers ("mostly
  true"; obscure `SQLITE_BUSY` cases exist); a reader that never stops starves checkpoints; the last connection to
  close checkpoints and deletes the WAL and `-shm`; a read-only WAL open needs existing `-shm` and `-wal`, or write
  access to the folder to create them, or `immutable`.
- **`rusqlite` 0.41.1 / `libsqlite3-sys` 0.39.1** (crates.io API, 2026-11-06). `libsqlite3-sys` has no `winsqlite3`
  feature: without `bundled` it links a library named `sqlite3` (`build.rs:385-390,472-527`). `bundled` compiles the
  SQLite 3.54.1 amalgamation (`sqlite3/sqlite3.c`, 9.6 MB of C) with `cc`, so each target needs a C compiler;
  sqlite.org/about.html: "With all features enabled, the library size can be less than 900KiB". The x64 tray is
  about 3.95 MB today (research.md, "Tray size"). Bundled size in Wakeshot's build: not measured (no build here).
- **No-SQLite signals.** None found. 1.x keeps busy state in memory; the only files that change with the state are
  the database, its `-wal` (any commit) and OpenCode's logs, whose lines are not a format. Parsing the SQLite file
  and WAL by hand was not considered further: it needs the WAL index and locking rules to read a live file safely.

### Options

| Option | Size added | Build | Version | Risk |
|---|---|---|---|---|
| `winsqlite3.dll`, loaded at run time | 0 | none: about ten `extern "system"` functions via `LoadLibraryExW` and `GetProcAddress` (`Win32_System_LibraryLoader` is already on) | whatever Windows ships (3.53.4 here) | an old or missing DLL on some build: then no OpenCode signal, nothing else breaks |
| `rusqlite` with `bundled` | under 900 KiB by SQLite's own number, not measured | a C compile for x64 and ARM64 (MSVC or clang) | pinned 3.54.1 | new dependency pair, longer builds, our own SQLite security updates |
| No SQLite | 0 | none | n/a | no signal exists |

**Pick: `winsqlite3.dll`, loaded at run time.** Zero size and no C build fit architecture section 11; Microsoft
documents it for app use and updates it with Windows. Run-time loading (not the SDK import library) means a Windows
without it still starts the tray: the source reports `Unreadable`. Every query is plain SQL over columns that exist
since SQLite 3.8; the 1.x query needs JSON functions (built in since 3.38.0), so the reader checks
`sqlite3_libversion_number() >= 3038000`. Change it when Wakeshot needs SQLite for something heavier, or a supported
Windows build turns out to lack the DLL or JSON: then `rusqlite` with `bundled`. Decision for Haik's review.

### How to read safely

- Read only when `<db>-wal` exists. OpenCode opens the database in WAL mode for the whole process life, and the last
  connection deletes the WAL, so a missing `-wal` means no OpenCode has the file open and nothing can run. This also
  keeps Wakeshot from creating `-wal` and `-shm` in OpenCode's folder (probe above).
- Open with `SQLITE_OPEN_READONLY` (plus `SQLITE_OPEN_NOMUTEX`, one thread), `busy_timeout` 100 ms, run the fixed
  queries, finalize every statement, close. Never keep the connection or a statement open between reads: an open
  read would hold back checkpoints, and a held connection would stop OpenCode's last close from deleting its WAL.
  Never `immutable=1`: it is only safe when nobody writes.
- On `SQLITE_BUSY` keep the last answer and try at the next trigger. On a prepare error (a renamed column) report
  `Unreadable`.
- Trigger: a change in the data folder to `opencode.db-wal`, at most one read per second, plus the safety tick.

## Liveness without `service.json`

- **Image and start time, no handle.** The system process list (`NtQuerySystemInformation(SystemProcessInformation)`)
  gives every process's image name and creation time at Medium, no handle needed (verified in research.md,
  "Medium reader, High `claude.exe`"; already used by `wakeshot-windows/src/process.rs`).
- **Exit wait.** `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | SYNCHRONIZE)` works from Medium on a High target;
  one more right fails the open (verified, same section). So the watch thread can wait on each matching process.
- **Command line.** `NtQueryInformationProcess(ProcessCommandLineInformation = 60)` on that same handle returned a
  child's full command line here (probe above), but only from this High token with `SeDebugPrivilege`, so access
  from Medium to a High target is not verified. The class is not on Microsoft Learn, whose page says the function
  "may be altered or unavailable in future versions of Windows"; `windows` 0.62.2 binds both
  (`Wdk::System::Threading`, `ProcessCommandLineInformation = PROCESSINFOCLASS(60)`, feature
  `Wdk_System_Threading`, not on yet). Reading the command line from the process memory needs `PROCESS_VM_READ`, which
  Medium does not get on a High process (verified, same research.md section).
- **Pick: image plus start time, no command line.** A row counts while a live process with image `opencode.exe` or
  `opencode-cli.exe` started at or before the row's time (1.x: the message's `time_created`; 2.x: `time_busy`), with
  2 s of slack for clock steps. The writer always started before its own write, so a live session always passes; a
  row left by a crash fails as soon as every OpenCode process alive started after it, which is the usual case (the
  2.x service or the 1.x TUI is started again). Limit: a stale row still counts while some older OpenCode process
  lives (1.x: a second TUI; 2.x: a TUI client that outlived a crashed service). Change it when a live test shows
  that case in practice: then add the `serve` argument check for 2.x through the command line.

## Not covered

- The desktop app beyond its sidecar name; OpenCode installed by other means than npm (the exe name is
  `opencode.exe` in both npm platform packages; other installers not checked); `OPENCODE_DB` or a non-standard
  channel (the reader looks only for `opencode.db`); 2.x goals; 1.x background jobs.

## What is left

- A live test of both versions on this laptop with a free local model (both have local providers; not run): a long
  tool call, an Esc interrupt, a permission prompt, a crash (kill the process), a 2.x service restart mid-turn, a 2.x
  schedule, and a 2.x background shell job (whether its `job` row closes).

## Built (Part B, 2026-11-06)

`p4-opencode-source` Part B built the 2.x rules above; nothing of it was run against a real OpenCode.

- `crates/wakeshot-core/src/sources/opencode.rs`: three queries on `sqlite_master`, `session_v2` and `schedule` only
  (1.x not read, Haik 2026-11-06). Ids are taken only in OpenCode's shape (`ses` or `sch_` prefix, letters, digits,
  at most 128 characters; OpenCode makes `ses_` or `sch_` plus 26, `schema/src/identifier.ts`, `session-id.ts`,
  `schedule.ts:8-12`). Row times are wall ms (`Clock.currentTimeMillis`, `core/src/bus.ts:524`).
- `crates/wakeshot-windows/src/sqlite.rs`: the `winsqlite3.dll` reader, with a VFS shim that opens the database and
  its `-wal` read only and never creates them (`docs/research.md`, "OpenCode SQLite reader checks").
- An older 2.x whose `session_v2` has no `time_busy` (before migration `20261029073502_session_time_busy`) fails the
  working query, so it shows "update Wakeshot". Which 2.x release added the column: not verified.
- Still to check live (the live card): every item of "What is left", plus that the `-wal` and `-shm` exist whenever
  OpenCode 2.x has the file open (the reader answers nothing otherwise).
