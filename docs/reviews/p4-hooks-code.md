# Review: agent hooks code (security and safety)

Reviewer: `wakeshot-reviewer`, 2026-11-06. Checks the built code against `docs/agent-hooks.md` and its security review
(`reviews/p4-hooks-design-security.md`, `-verify.md`). Read only: no code changed, no git, no test run (the code was
enough for each finding; four cards build now). No real `.cursor` or `.gemini` folder was read or touched.

## Files read, and when

| File | Read at | File time |
|---|---|---|
| `docs/agent-hooks.md` | 16:37 | 16:34 |
| `docs/tasks/p4-agent-hooks-install.md`, `p4-cursor-source.md` (Result) | 16:38, 16:44 | |
| `crates/wakeshot-windows/src/agent_hooks.rs` (code all, tests in part) | 16:38 to 16:48 | 16:12 |
| `crates/wakeshot-windows/src/hooks.rs` (uninstall step, `RealHookEnv`, data folder step) | 16:40, 16:50 | 16:12 |
| `crates/wakeshot-windows/src/app/startup.rs` (`add_agent_hooks`) | 16:40 | 16:12 |
| `crates/wakeshot-cli/src/hook.rs` (code all, tests in part) | 16:41, 16:46 | 16:29 |
| `crates/wakeshot-cli/tests/hook.rs` | 16:45 | 16:28 |
| `crates/wakeshot-core/src/sources/hooked.rs` (code all) | 16:42 | 16:28 |
| `crates/wakeshot-core/src/sources/cursor.rs` | 16:44 | 16:08 |
| `crates/wakeshot-windows/src/process.rs` (`image_path`, `command_line_contains`, `current_above_medium`, `harden_dll_search`, `spawn_outside_tree`) | 16:43 | 15:33 |
| `crates/wakeshot-windows/src/store.rs` (`write_atomic`, `temp_target`, `local_app_data`, `read_whole`, `NamedMutex`) | 16:42 | 15:33 |
| `docs/research.md`, "Cursor hook payload form" | 16:49 | |
| `crates/wakeshot-core/src/sources/antigravity.rs` | 16:48 | 16:48 (being written) |

Antigravity: `agent_hooks::AGENTS` and the CLI's `AGENTS` held Cursor only when read. `sources/antigravity.rs` was
being edited by `p4-antigravity-source` while read; it is not reviewed here.

## Counts

Blockers 0. Major 1. Minor 7.

## Major

### M1. The runtime privacy check runs only when the walk finds an agent process

`crates/wakeshot-cli/src/hook.rs:315-326` (also `:295-304`).

What breaks: `hook()` returns at "no Cursor process above the hook; nothing written" before it calls
`input_on_command_line`. It also returns before it when the process list fails. So the parent's command line is
checked only when `walk.agent` is `Some`. The design says the check runs "in every hook run" ("Privacy", "How the
build checks it", point 2), and it is the only backstop if a later Cursor puts the payload on a command line.

Scenario: `research.md` ("Cursor hook payload form") found that the one Cursor path that defaults to the command line
(`argv_heredoc`) is the agent host daemon's `CliHooksExecutor`, which "already reads `config.pluginHooks`, so a later
version could feed plugin hooks to it; the hook's runtime check catches that". Whether the daemon runs as `Cursor.exe`
is **not verified** (design, Facts table). If it runs as any other image (a bundled `node.exe` outside
`%LocalAppData%\cursor-agent\`, a renamed helper), the walk finds nothing, the hook returns early, and the check never
runs. Every prompt and the user's email then go into PowerShell's command line, and so into process audit logs, for as
long as the plugin stays, and Wakeshot never notices. The two changes (a new host process and the command-line form)
come together, so the backstop fails in the most likely leak case.

Fix: run the check on `walk.parent` before the `walk.agent` gate (and when the list fails, read the parent from the
hook's own `ProcessBasicInformation`, or skip only then). On a hit with no agent found: at Medium call
`env.remove_hooks`; above Medium hand off `hook <agent> remove` (no checked values needed); one log line; no
`blocked.json` (no process to name), or name the parent. Add a test: a fake with no agent image above the hook and
`command_line = Ok(true)` must still remove the plugin.

## Minor

### m1. The uninstall's agent step can spend 5 s per agent before the restore

`crates/wakeshot-windows/src/hooks.rs:227`, `crates/wakeshot-windows/src/agent_hooks.rs:57`, `:527`, `:586`.

What breaks: above Medium, each agent gets its own copy and its own wait of up to `COPY_WAIT_MS` (5 s), one after the
other, before `restore`. Restore retries are counted from the hook's start (`UNINSTALL_LIMIT_MS` 25 s, Velopack kills at
30 s per its docs). With Cursor and Antigravity that is up to 10 s less for restore retries of Haik's power values. One
try still always runs, so this is a smaller margin, not a lost restore. A copy still running after its wait can also
open the `cli` log after the data folder step deleted it, and so leave `%LocalAppData%\Wakeshot\logs\` behind
(`LogWriter::open` makes the folder, `log.rs:86`).

Fix: start every agent's copy first, then wait once with one shared 5 s budget (or about 3 s). Keep the step before
the restore, as designed.

### m2. "One log line" about hooks left behind is deleted moments later

`crates/wakeshot-windows/src/hooks.rs:228-236` with `clean_on_disk` `:837-884`; texts at `agent_hooks.rs:519-554` and
`:689-696`.

What breaks: when the restore succeeds, the data folder step is `DeleteAll`, which deletes the logs too. So the lines
"hook folder not removed: its copy could not start under Explorer", "could not be removed ... or renamed ...; delete it
yourself" and the Medium copy's own `cli` lines are gone right after they are written. The user is left with a Cursor
plugin whose hooks run a deleted exe (exit 127 and a PowerShell start on every prompt), and no trace. The design
promises "one log line" in these cases; only the README line remains.

Fix: when the agent step leaves a folder in place, make the data folder step keep the logs (as
`KeepJournalAndLogs` does), or accept it and change the design text to say the README is the only pointer.

### m3. One unreadable staging folder stops the uninstall from removing the live plugin

`crates/wakeshot-windows/src/agent_hooks.rs:653` (`clean_staging(...)?`), inside it `:457-473` (`list`, `kind`,
`look` with `?`).

What breaks: `remove_one` cleans `.wakeshot-new-*` folders first and returns on any error there. A staging folder that
cannot be listed or a file in it that cannot be opened (a deny ACL, a sharing violation, a folder another program named
`.wakeshot-new-x`) makes the whole agent's removal fail with "hook folder not removed", and `wakeshot\` stays with live
hooks that name the exe the uninstall deletes. The same at a start skips the rewrite. A leftover should never block
the main folder.

Fix: in `clean_staging`, turn each folder's error into one log line and go on; run the main folder's removal anyway.

### m4. A second `.wakeshot-removed` cannot be made, so the plugin stays loaded

`crates/wakeshot-windows/src/agent_hooks.rs:681`, `move_folder` `:769-780`; tested as intended in
`uninstall_renames_a_folder_it_cannot_empty` (`:1850-1856`).

What breaks: after one install and uninstall left `.wakeshot-removed`, a later uninstall of a folder the user changed
cannot rename (183) and leaves `wakeshot\` as it is. If the changed file is `hooks\hooks.json` (for example only its
`timeout` edited), its command still runs `wakeshot.exe`: after the uninstall, exit 127 on every prompt. The runtime
privacy removal (`remove_command`) has the same end, so a command-line leak would go on past "the one prompt".

Fix: rename to the first free name of `.wakeshot-removed`, `.wakeshot-removed-2`, ... (still never replacing a folder).
Update the README line in `agent-hooks.md` ("README lines") to match.

### m5. The blocked path leaves other chats' files with no end to come

`crates/wakeshot-cli/src/hook.rs:340-343`, `:513-516`.

What breaks: on a command-line hit the hook writes `blocked.json`, keeps no ids, and removes the plugin. From then on no
`stop` or `sessionEnd` runs for that agent. A chat of another tab that was mid-turn keeps its `working` file, and it
holds the PC awake while the named process lives (the extension host: a whole Cursor session, on AC). The battery floor
still protects on battery.

Fix: on the blocked path, also delete this agent's chat files (`<uuid>.json` in `agents\<agent>\`, by the id rule)
under the mutex, since the plugin that would end them is gone.

### m6. `agent_hooks::remove` is public and does no level check

`crates/wakeshot-windows/src/agent_hooks.rs:626-635`.

What breaks: nothing today: only `on_uninstall` (after its own check) and `remove_command` call it. But the design
plans a caller ("Source turned off": "the tray runs the same removal at start"), and a tray can run above Medium. A
future caller that skips the check deletes in user folders while elevated, the case the whole design avoids.

Fix: make `remove` private, or give it the `above_medium` argument and the `medium()` check, as `at_start` has.

### m7. Docs that no longer match the code

- `docs/agent-hooks.md`, step 3: "No match: write nothing, one log line. Then the runtime transport check" reads as if
  the check always runs; the code skips it (M1). Fix the code, then the text stays true.
- `docs/agent-hooks.md`, step 4: a copy that is not Medium "writes nothing, one log line". The code logs nothing
  (`hook.rs:440-442`), as the "Built" note says for elevated hooks. Change step 4 to "no log line".
- `docs/tasks/p4-agent-hooks-install.md`, "Found outside scope" 2 and "Open items": "an elevated uninstall removes
  none" until the CLI calls `remove_command`. That landed (`hook.rs:216-218`, `:256-258`). Only item 1 (the
  `tray_mode` call line) is still open; no tray calls `startup::add_agent_hooks` yet (grep: only its test).

## Checked and found sound

- Every agent folder write and delete goes by the fixed names; each part is checked with `symlink_metadata` and any
  reparse point stops the work; reads use `FILE_FLAG_OPEN_REPARSE_POINT`; deletes are `DeleteFileW` and
  `RemoveDirectoryW`, never recursive; renames never replace (`MoveFileExW` without `REPLACE_EXISTING`). A crafted
  marker can only make Wakeshot delete or rewrite its own fixed names inside `wakeshot\`; a crafted record can only
  stop adds. At Medium a race or swap gains nothing.
- Above Medium: `at_start`, `remove_command`, `on_uninstall` and the hook write or delete nothing in user folders, and
  the hook logs nothing (`Lines` gives `Quiet`). The copy re-checks its token and every value (`Checked::parse`: UUID
  lowercase, turn id set, digits, fixed image key and op per event). The deadline is set before the hand-off, so it
  ends no later than the elevated hook's wait.
- The hook: DLL search limited first; the panic hook prints `{}` and exits 0; every path ends in `{}` and
  `ExitCode::SUCCESS`; input capped at 64 MiB then drained; `serde_json`'s `ignore_value` is iterative (read in
  1.0.154), so deep input cannot overflow the stack; errors log class and place only; the version is filtered.
- State files: ids by UUID shape only, so no device name or traversal; `write_atomic` temp files open with
  `CREATE_NEW`; prune deletes only version 1 files of gone processes and old temp names, plain files only.
- Walk: a parent counts only if listed and not younger than its child; `image_path` and `command_line_contains` re-check
  the start time after the open, so a reused pid never answers.

Not verified here (live tests own them): whether the editor skips dot-named folders; whether Cursor's file watcher
handle on `hooks\` makes the uninstall's removes or rename fail; whether the daemon runs as `Cursor.exe`.
