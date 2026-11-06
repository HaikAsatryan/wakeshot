# Agent hooks

Design, 2026-11-06 (card `p4-hooks-design`), with the fixes of its security review
(`reviews/p4-hooks-design-security.md`). Later the top part becomes the user reference, like `hold-files.md`.

Some agents leave no file that tells "working" from "idle". For those, Wakeshot adds its own small hook: a folder it
owns, which the agent loads by itself, that runs one Wakeshot command when a turn starts and when it ends. Haik's rule
(`product.md`, "What keeps the PC awake", Hooks, 2026-11-06): never edit the user's own settings, never for Claude
Code; Wakeshot adds the hooks by itself when it finds the agent, says so once in a notice, lists them in the README,
and removes them at uninstall.

Marks: **verified** = read in the agent's shipped code or official docs (version and file given). **not verified**
= inferred or not checked. Nothing was installed or run: Cursor CLI `2026.11.05-86047df` (Windows package zip) and
Cursor editor `3.29.9` (commit `306d153d51d7`, Linux `.deb`; its app JS is shared with Windows) were unpacked as data.

## What Wakeshot adds (user view)

| Agent | Folder Wakeshot adds | Runs on | Removed |
|---|---|---|---|
| Cursor (editor and CLI) | `%USERPROFILE%\.cursor\plugins\local\wakeshot\` | prompt sent, agent stopped, chat ended | at uninstall; or delete the folder yourself |
| Antigravity (CLI and IDE) | `%USERPROFILE%\.gemini\config\plugins\wakeshot\` | each model call, agent stopped | the same |

- What runs: one command for every agent, `wakeshot.exe hook <agent> <event>`, from Wakeshot's install folder. It
  writes one small file per chat in `%LocalAppData%\Wakeshot\agents\<agent>\` while the agent works, and deletes it
  when the agent stops.
- What it keeps: the chat's id, the turn's id and the agent's process. Never the prompt, file names or your email.
- While a hook runs, the agent hands it your prompt, and in some modes writes it to a temp file or a command line
  first (Cursor's CLI uses a temp file it deletes after the hook). Wakeshot's hook keeps none of it. Wakeshot does not
  add a hook to an agent that would put prompts on a command line (see "Privacy").
- If you delete the folder, Wakeshot does not add it again. To get it back: reinstall Wakeshot, or delete
  `%LocalAppData%\Wakeshot\agent-hooks.json` and restart Wakeshot.
- Each agent loads the folder at its next start (Cursor also at Developer: Reload Window).

## Cursor

### Facts

| Fact | Where read | Mark |
|---|---|---|
| A local plugin is a folder in `~/.cursor/plugins/local/<name>/` with `.cursor-plugin/plugin.json`; loaded at start or Reload Window, "if local plugin imports are allowed" (off by default on Enterprise) | `cursor.com/docs/plugins`, read 2026-11-06 | verified (docs) |
| Manifest needs only `name` (lowercase, kebab case); hooks come from `hooks/hooks.json` unless the manifest says otherwise | `cursor.com/docs/reference/plugins` | verified (docs) |
| The CLI reads every folder in `<home>\.cursor\plugins\local\`, skips names starting with `.`, and rejects a link whose target is outside that folder | CLI `index.js`, `loadUserLocalPlugins` | verified (code) |
| Hook file `{ "version": 1, "hooks": { "<event>": [ { "command": "...", "timeout": <s> } ] } }`; Cursor's own plugin writer emits this exact form | CLI `index.js`; docs `cursor.com/docs/agent/hooks` | verified |
| Plugin hooks run after enterprise, team, project and user hooks; all run, in parallel; results merge per field; `continue` is false if any hook says false; `followup_message` last wins | CLI `619.index.js` `executeAllAndMerge`; `index.js` merge `Q` | verified (code) |
| Editor fires plugin `beforeSubmitPrompt` and `stop`: `hasHookForStep` counts plugin hooks | editor `workbench.desktop.main.js`, `hasHookForStep`, `triggerStopHook` | verified (code) |
| **CLI interactive loop fires `beforeSubmitPrompt` and `stop` only when the user's or the project's `hooks.json` has that event**; plugin hooks alone are not counted there. `sessionStart` and `sessionEnd` use `hasHooksForStep`, which counts plugins | CLI `3026.index.js` (the gates before `executeHookForStep`) | verified (code), **not run** |
| `stop` carries `status` (`completed`, `aborted`, `error`) and the turn's `generation_id`; the editor compares it with the chat's latest turn ("a newer turn owns the chat") | editor `triggerStopHook`; CLI `3026.index.js` (three `stop` calls: completed, aborted, error) | verified (code) |
| `beforeSubmitPrompt` input holds the prompt text and attachments; every input holds `conversation_id`, `generation_id`, `user_email`, `workspace_roots`, `transcript_path`, `cursor_version` | docs; CLI `executeHookForStep` | verified |
| On Windows the command runs in PowerShell. CLI: payload in a temp file `%TEMP%\cursor-hooks-<random>\payload.json`, then `$OutputEncoding` set to UTF-8 and `Get-Content -Raw -Encoding UTF8 <file> \| & { $input \| <command> }`. Other mode: the payload as a here-string in the PowerShell command line | CLI `619.index.js` `executeCommandScript`; `index.js` `dt`, `it` (`"stdin"` gives the temp file) | verified (code); the editor's mode **not verified** |
| A command that is not found ends with exit 127; any exit but 2 fails open (the action goes on); exit 2 blocks; empty stdout counts as a failed hook; default timeout 60 s | CLI `index.js` `lt`, `R=60`; editor `extensionHostProcess.js` has the same exit 127 code | verified (code) |
| Plugin `stop` hooks run in the workspace root, other plugin hooks in the plugin folder; `CURSOR_PLUGIN_ROOT` is set | CLI `619.index.js`; docs | verified |
| The editor runs hook commands from its extension host or its agent host daemon, both `Cursor.exe` processes | the spawn code is in `extensionHostProcess.js` and `agent-host-daemon/dist/bin/daemon.cjs`; the image name by VS Code convention | **not verified** |
| Cursor watches the hook files and reloads them on change | docs | verified (docs) |

### The folder

`%USERPROFILE%\.cursor\plugins\local\wakeshot\`, the profile folder from `FOLDERID_Profile`:

```
wakeshot\
  .cursor-plugin\plugin.json    {"name":"wakeshot","version":"1.0.0","description":"Lets Wakeshot keep the PC awake while the agent works. Added by Wakeshot.","author":{"name":"Wakeshot"},"homepage":"https://github.com/HaikAsatryan/wakeshot"}
  hooks\hooks.json              the three hooks below
  wakeshot-hooks.json           Wakeshot's marker: {"version":1,"wakeshot":"<version>","files":{"plugin":"<FNV-1a 64>","hooks":"<FNV-1a 64>"}}
```

The file set is fixed in Wakeshot's code, per agent: for Cursor exactly `.cursor-plugin\plugin.json`,
`hooks\hooks.json` and `wakeshot-hooks.json`, in exactly the folders `wakeshot\`, `wakeshot\.cursor-plugin\` and
`wakeshot\hooks\`. The marker names files by fixed keys (`plugin`, `hooks`), never by path, and holds only the hash
of what Wakeshot wrote. A marker with any other key, or a folder holding any other file or folder, is not "ours,
unchanged" (see "Adding"). Wakeshot never reads a path out of the marker.

Why a local plugin: it is the only form both Cursor apps load that needs no edit of a file the user owns. The user
hooks file `~/.cursor/hooks.json`, project `.cursor\hooks.json` and Claude's `~/.claude/settings.json` (which Cursor
also reads) are the user's own: never touched.

`hooks.json`, with `<exe>` the installed `wakeshot.exe`:

```json
{ "version": 1, "hooks": {
  "beforeSubmitPrompt": [ { "command": "'<exe>' hook cursor beforeSubmitPrompt", "timeout": 5 } ],
  "stop":               [ { "command": "'<exe>' hook cursor stop", "timeout": 5 } ],
  "sessionEnd":         [ { "command": "'<exe>' hook cursor sessionEnd", "timeout": 5 } ] } }
```

- `<exe>` is `%LocalAppData%\HaikAsatryan.Wakeshot\current\wakeshot.exe`: `current` keeps its path across updates
  (tested, research "Velopack checks for Wakeshot", point 3).
- A single-quoted path is the one shape that works in both of Cursor's PowerShell forms (`& <command>` and
  `$input | <command>`, where Cursor adds `& ` before a quote). A script block or an `if` breaks one of them. So the
  command cannot test for the exe first; a missing exe gives exit 127 (below).
- PowerShell single quotes expand nothing (`$`, backtick). An ASCII `'` in the path (O'Brien) is doubled: PowerShell
  reads `''` as one quote, and Cursor's own path scan skips a doubled `'` (CLI `index.js`). Wakeshot refuses to add
  hooks when the path holds U+2018, U+2019, U+201A or U+201B (PowerShell counts them as quotes too, from its language
  spec, not re-read today) or a control character: one log line, no hooks, no notice.
- Non-ASCII user names (Armenian, Russian): Cursor and PowerShell take UTF-16 command lines, so likely fine; how
  Cursor's shell executor hands over the command text was not traced. If any step decoded it as a legacy code page,
  `ё` (UTF-8 `D1 91`) and `ւ` (`D6 82`) would turn into U+2018 and U+201A and break the quoting. The live test runs
  the exe from a folder named with `ё`, `ւ` and a space.
- `timeout` 5 s caps the wait at 5 s instead of 60 s, whatever happens to the exe.
- No `failClosed`, no `matcher`, no `followup_message`, no `permission`. Exit code is always 0.

### Events and states

| Event | Wakeshot does | State |
|---|---|---|
| `beforeSubmitPrompt` | writes the chat's file (replaces it) | working |
| `stop` (any `status`) | deletes the chat's file if it names this turn's `generation_id` | idle |
| `sessionEnd` | deletes the chat's file | gone |

- Idle and gone are "no file": the source sees only working chats.
- Waiting for approval: Cursor sends no hook event for it (the CLI maps Claude's `Notification` and
  `PermissionRequest` to nothing, `index.js`). So an approval wait counts as working, as for Codex (`product.md`,
  "Answers to the track plan questions"). The battery floor still protects. Product question 2.
- Subagents run inside the parent turn, which ends with the parent's `stop`. `subagentStart`/`subagentStop` are not
  hooked. Change it when a live test shows a subagent still running after the parent's `stop`.
- The `generation_id` check keeps a late `stop` of turn N from deleting turn N+1's file. That the prompt hook and the
  stop hook carry the same id for one turn: likely (both are the turn's id in the editor code), **not verified**; the
  live test checks it. If they differ, `stop` would never match: change the rule then to "delete unless the file
  names another turn and is under 2 s old".

### The hook command

`wakeshot.exe hook <agent> <event>`: one CLI command for every agent, not shown in `wakeshot --help`. Per agent it
knows only a small table: the input field names, the events and the process images. The steps for Cursor
(Antigravity: `conversationId` and `fullyIdle` instead, and `agy.exe`, see "Antigravity"):

0. First, before anything else: `SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32)`, since a plugin `stop`
   hook runs with an untrusted repo as its working folder; and a panic hook that prints `{}`, flushes and calls
   `process::exit(0)`, logging no input (the release profile has `panic = "abort"`, so `catch_unwind` cannot help).
1. Stream stdin through `serde_json` into a struct of four fields only: `conversation_id`, `generation_id`,
   `hook_event_name` and `cursor_version`. Every other field (the prompt, attachments, email, paths) is skipped by
   the parser without being stored. At most 64 MiB is read, then stdin is drained to its end, so Cursor's pipe never
   breaks. Cursor sends UTF-8 (the CLI's temp-file form sets it; verified); a byte order mark is skipped; input that
   does not parse is unreadable (below).
2. Check the input: `conversation_id` must have the UUID shape (8-4-4-4-12 hex digits) and is lowercased; it becomes
   the file name, so device names (`CON`, `COM1`) and case twins cannot occur. Antigravity's is a UUID (its docs);
   Cursor's looks like one, **not verified**: the live test checks it, and another shape shows as unreadable input
   (the "update Wakeshot" notice), not as a silent gap. `generation_id`: at most 80 of `A-Z a-z 0-9 - _`, or empty.
   `hook_event_name`, if present, must equal the event in the command; else the input counts as unreadable.
3. Find the agent process: list processes with their parents (`process::list`) and walk up from the hook's parent,
   at most 10 steps. Each parent must be listed and must have started at or before its child, so a reused pid ends
   the walk. Take the nearest match: the first `Cursor.exe` (editor, label "Cursor"), or the first `node.exe` or
   `cursor-agent.exe` whose full image path lies under `%LocalAppData%\cursor-agent\` (CLI, label "Cursor CLI";
   install layout from `research/other-agent-signals.md`). The nearest is the process that ran the hook (an
   extension host, an agent daemon or the CLI), never a farther window or daemon above it. The path comes from
   `process::image_path` (`QueryFullProcessImageNameW`, `PROCESS_QUERY_LIMITED_INFORMATION`). Then, match or not, the
   runtime transport check on the hook's parent (see "Privacy", "How the build checks it"). No match and nothing on
   the command line: write nothing, one log line.
4. File work runs only at Medium integrity or lower. The hook reads its own token's integrity level:
   - Medium or lower: it does step 5 itself.
   - Above Medium (an agent started from an admin terminal; every agent shell on the test laptop is High): it writes
     nothing. It starts a copy of itself at Medium through `process::spawn_outside_tree` (Explorer's token), with the
     checked values as arguments: `hook <agent> <event> --checked <conversation> <generation> <pid> <procStart>
     <label> <state> <deadline>`, all from fixed character sets. `deadline` is the elevated hook's start plus 2 s, a
     FILETIME of the wall clock. It waits for that copy at most 2 s. The copy checks its own token again; if it is not
     Medium (no Explorer, so `spawn_outside_tree` fell back to a normal child), it writes nothing and logs nothing. The
     copy runs the step 2 checks again on every value (UUID shape and lowercase, the generation's characters, digits
     for pid, start and deadline, the label and state from their fixed lists, the agent and event from the known
     table), since any process of the user can run it with any arguments; one failed check: nothing written, one log
     line. A copy that runs past its deadline drops a "working" write, so a slow copy can never land after the turn's
     `stop` (the wait handle has no right to end the copy); deletes still apply. Reason for the hand-off: the
     folders below inherit the profile's rights, so any Medium process of the same user can swap them for a junction
     or plant hard links in them; an elevated write or delete there would hand it a
     write and delete primitive it does not have without Wakeshot (review major 1). At Medium a swap gains nothing.
     Cost: one more process start for elevated agents.
5. Under the mutex `Local\Wakeshot.AgentHooks` (wait at most 1 s, then go on without it): apply the event to
   `%LocalAppData%\Wakeshot\agents\cursor\<conversation_id>.json` (create the folders; `store::write_atomic`, whose
   temp file opens with `CREATE_NEW`: contract change 6). On `beforeSubmitPrompt` also prune: delete a file in that
   folder only if it parses as version 1 and names a process that is not running, and temp files of
   `write_atomic`'s own name shape older than 1 minute. Nothing else is ever deleted.
6. Print `{}` and exit 0. Always, also after any error. No path exits 2, which is Cursor's block code. `{}` merges to
   nothing (per-field merge), so it never changes another hook's answer, and a non-empty JSON reply keeps Cursor from
   counting the hook as failed.

Unreadable input (step 1 or 2) with a found agent process writes `agents\cursor\problem.json` (below) instead. Every
failure is one log line in `logs\wakeshot.log` (process `cli`, area `agent-hook`) with fixed text, the event name
and an error code; never input bytes. A parse error logs only `serde_json::Error::classify()` and its line and
column, never the error's text, which can quote an input value. Target: under 50 ms for the exe; PowerShell's own
start is Cursor's cost.

### The files it writes

`agents\cursor\<conversation_id>.json` (the same format for every agent, in its own folder):

```json
{ "version": 1, "pid": 23816, "procStart": "134381211391959336", "generation": "8f0c...", "label": "Cursor",
  "state": "working" }
```

`state` is `working` or `background` (Antigravity only, below); Cursor always writes `working`.

`agents\cursor\problem.json`: `{ "version": 1, "pid": 23816, "procStart": "...", "problem": "input" }`.

- `pid` and `procStart` are the agent process from step 3, never the hook's own: the hook and its PowerShell are
  gone within a second.
- `label` is `Cursor` or `Cursor CLI`, from the image found.
- The tray never writes these files and never deletes them; the Medium hook prunes them, and the uninstall deletes
  the data folder.

### How the source reads them

`sources::cursor` (core, `Owner: p4-cursor-source`):

- `wanted()`: one `WantedFile::Folder { dir: <data>\agents\cursor, accept: name ends in ".json" }` (temp names from
  `write_atomic` do not end in `.json`).
- `feed`: `Whole` parses version 1; a newer `version` is ignored with a log line; a bad file gets `RetryOnce`, then
  no signal; `Gone` drops it.
- `sessions()`: one `SessionFacts` per chat file: key `(Cursor, conversation_id)`, `Liveness::Process { pid, start }`,
  `Activity::Working` (for `state` `background`: `Activity::Idle` with `open_tasks` 1), no timers, `FormatHealth::Ok`.
  The label is one of the source's own fixed strings (`Cursor`, `Cursor CLI`; `Antigravity`, `Antigravity CLI`),
  picked by the file's value; any other value gives the agent's plain name. File text never reaches the tooltip. `sources::antigravity` reads `agents\antigravity\` the same way. `problem.json` gives one
  `Activity::Idle` session with `FormatHealth::Unreadable("Cursor hook input")`, so the "update Wakeshot" notice
  shows once per start while that process lives. `blocked.json` (the runtime transport check, "Privacy") is read the
  same way.
- No new `WantedFile` or `Liveness` variant is needed.

**Built** (`p4-cursor-source`, 2026-11-06): "The hook command", "The files it writes" and this part, in
`sources::hooked` (shared: the `Agent` table type, the parse, the id rules, the event rules with Antigravity's idle
stop, the files, the walk, the checked values, a generic `HookedSource`), `sources::cursor` (`AGENT`, `LABEL`,
`CLI_LABEL`, `source(data_root)`) and the CLI's `hook.rs`. Live test 0 is done by code reading: the editor 3.27.12
passes the input through a temp file on every path that runs a plugin hook, so the Gate holds (`research.md`, "Cursor
hook payload form"). Where the build differs from the text above:

- The runtime check searches the bare key `conversation_id`, not `"conversation_id"`: Node writes a `"` inside an
  argument as `\"`, so the quoted needle would miss a here-string passed through Node (checked with Node 26). Found,
  the hook writes `blocked.json` and calls `agent_hooks::remove_command`.
- The checked values are `<chat> <turn> <pid> <procStart> <image key> <op> <deadline>`: the image key is `editor` or
  `cli` (no space), the op is `working`, `background`, `none` (the event's delete), `input` or `command-line` (the
  two problem files, which carry empty ids), and the deadline is wall ms since 1970, the same clock as a FILETIME.
- An elevated hook writes no log line either: the log folder is in the same data folder a Medium process can swap.
  Its Medium copy logs. A hook with nothing to say opens no log.
- A newer `version` in a file is ignored with no log line: sources have no log (core). A bad file gets `RetryOnce`,
  then no session and no notice.
- `hook <agent> remove` hands the agent word to `agent_hooks::remove_command` as given and reads no input.
- For the tray: `tray_mode::hooked_sources` (`p4-antigravity-source`).

**Review fixes** (`p4-live-cursor`, `reviews/p4-hooks-code.md`):

- The runtime check runs before the agent gate (M1). Found with no agent process above the hook: nothing written and
  no `blocked.json` (no process to name), one log line, the plugin removed (above Medium: through the Medium copy's
  `hook <agent> remove`). When the process list fails, the hook's parent has no name, so the check cannot run: one log
  line, nothing written; the next hook run checks again (`process` has no call for one process's parent; change it
  when one lands).
- On the blocked path, and in every Medium `hook <agent> remove`, the agent's chat files (`<chat id>.json`, by the id
  rule) are deleted under the mutex: with the hooks gone no `stop` would end them (m5). `problem.json` and
  `blocked.json` stay.

### Cursor's `main.log`: dropped

The editor's own wakelock lines in `main.log` (`research/cursor-gemini-signals.md`) are not used, not even as a
second signal. Reasons: they are log text with no contract (wording, level and rotation not verified); they name no
chat, so they cannot be matched to a hook file; a second parser and a second live test cost more than they give. What
they would add: ending a hold during an approval wait (the editor releases its lock then), and ending a hold whose
`stop` never came. Change it when the live test shows `stop` missing in a real case (Esc, a crash of the agent
runtime), or Haik wants approval waits not to hold for Cursor (product question 2).

## Adding

Only an installed tray adds or changes hooks: one whose exe is in Velopack's `current` folder. A tray run from
`target\`, a dry run, the CLI and the guard never touch an agent's folders. A Store (MSIX) build adds none, since
MSIX has no uninstall hook to remove them (change it when a Store build ships with a way to remove them).

When: once at each tray start, on the IO thread, after the data folder exists (`app/startup.rs`). Cursor is present
when `%USERPROFILE%\.cursor\` exists: both Cursor apps create it (the CLI's config home, verified; the editor's
`projects\` transcripts, verified in `research/cursor-gemini-signals.md`). Change it to "also when the watch thread
first sees a Cursor process" if users install Cursor and expect it to work before the next sign-in.

Wakeshot's record is `%LocalAppData%\Wakeshot\agent-hooks.json`: `{ "version": 1, "cursor": "added" | "removed",
"antigravity": ... }`. What the tray does for Cursor (Antigravity the same, with its own folders):

| `.cursor` | Plugin folder | Record | Action |
|---|---|---|---|
| missing | any | any | nothing |
| exists | missing | none | add (below), record `added`, one notice |
| exists | missing | `added` | the user removed it: record `removed`, one log line, no notice; never add again |
| exists | missing | `removed` | nothing |
| exists | ours, unchanged, same as this version | any | nothing; record `added` |
| exists | ours, unchanged, older content | any | rewrite the fixed files, one log line, no notice; record `added` |
| exists | ours, changed by someone | any | leave it all, one log line per start; record `added` |
| exists | not ours | any | never touch it, one log line |

- "Ours": `wakeshot-hooks.json` parses as version 1 with only the fixed keys. "Unchanged": the folder holds exactly
  the fixed files and folders, nothing more, and each fixed file hashes to the marker's value. An added file (a
  `rules\`, `skills\` or `mcp.json` that another program dropped in) makes it "changed by someone": Cursor would load
  it under Wakeshot's name, so Wakeshot never calls such a folder its own again. The hash finds edits, not attacks.
- All reads, writes and deletes go by the fixed names only, and each part of each path (`.cursor`, `plugins`,
  `local`, `wakeshot`, `.cursor-plugin`, `hooks`, the file) must be a plain folder or file, no reparse point; else
  nothing is touched, one log line.
- Only at Medium: a tray whose token is above Medium (started from an admin terminal) touches no agent folder, one
  log line. The installed tray starts from the Run value or from Velopack at Medium.
- Add: create `.wakeshot-new-<pid>\` in the agent's own folder (`.cursor\`; Antigravity `.gemini\config\`), outside
  the folder the agent loads plugins from, with "fail if it exists" (`CreateDirectoryW`), write the fixed
  files into it, then rename it to `wakeshot`. Cursor never sees half a plugin. Rewrite: each fixed file with
  `store::write_atomic`. A staging folder left by a crash: at each start and at uninstall, a `.wakeshot-new-*` folder
  that holds only the fixed files, each equal to what this version writes or to its own marker's hash, is removed by
  the fixed names, in the agent's own folder and in the plugin folder's parent (where test builds staged before).
  Why outside: `agy` 1.5.2 lists and runs dot-named plugin folders (`p4-live-antigravity`, live), so a staging or
  removed folder in its plugin folder would load; Cursor's CLI skips dot names (code), and one rule serves both.
- Never fight the user. Deleting the folder is the clearest "no" a user can give, and Cursor's own plugin switch (if
  the user turns it off there) keeps the folder, so Wakeshot does nothing against it either. A reinstall deletes the
  data folder and so starts as on a new PC, like the Run value's startup switch (`p6-uninstall-startup-approval`).
  Product question 3 asks whether a menu item should bring them back.

The notice, once, when Wakeshot adds the folder (`Notice::HooksAdded { agent: "Cursor" }`, quiet, no click):

> **Wakeshot now works with Cursor**
> It added a small hook to Cursor's plugins, so the PC stays awake while Cursor's agent works. It works from
> Cursor's next start.

## Removing

- **Uninstall** (`hooks.rs`, `on_uninstall`): a new step `remove_agent_hooks` right after the notice app id, before
  the restore: the hook files name an exe the uninstall deletes, the same reason the Run value goes first.
  - At Medium it removes in-process. Above Medium (`winget uninstall` from an admin terminal: Velopack's hook keeps
    that token, not tested) it starts `wakeshot.exe hook <agent> remove` at Medium through `spawn_outside_tree` for
    every agent first, then waits at most 5 s for them all; if a copy is not Medium, nothing is removed, one log line.
  - For each agent folder that is ours, by the fixed names only: delete each fixed file whose hash still matches,
    then the marker, then `hooks\` and `.cursor-plugin\` if empty, then `wakeshot\` if empty. Never `local\` or
    `plugins\`, never a recursive delete, never through a link, never a path read from the marker.
  - If a fixed file cannot be deleted (changed by the user, or held open by Cursor), the folder is renamed to
    `wakeshot-removed` in the agent's own folder (`.cursor\wakeshot-removed`; Antigravity
    `.gemini\config\wakeshot-removed`), or the first free one of `wakeshot-removed-2` to `-9`, outside the folder the
    agent loads plugins from (`agy` 1.5.2 loads dot-named folders there, live), so the plugin stops loading and the
    user's edits stay on disk. The hook file is deleted first when it is Wakeshot's, so what stays runs nothing.
    If that rename fails too, one log line; the README's uninstall line says which folder to delete. When the restore
    then succeeds, the data folder step deletes that line with the logs, so the README is the only pointer (review m2,
    accepted: keeping the logs needs a change of the `HookEnv` contract for a case that needs a changed or held file
    and a failed rename; change it when a user reports hooks left after an uninstall).
  - The `.wakeshot-new-*` staging rule above runs here too. The record goes with the data folder step.
- **Updated hook**: nothing. The command path does not change, and the next tray start rewrites older content.
- **Source turned off** (a later version that drops the Cursor source, or a switch if Haik wants one): the tray runs
  the same removal at start and records `removed`. Automatic off does not remove anything: it is a pause.
- **The `wakeshot hook` command form is frozen**: every later version accepts `hook <agent> <event>`, and an agent or
  event it does not know prints `{}` and exits 0, so hooks left by any version never fail. `remove` and `--checked`
  are Wakeshot's own forms (above); anyone of the same user can run them, which gives no more than deleting the
  folder or writing a state file by hand.

**Built** (`p4-agent-hooks-install`, 2026-11-06): "Adding" and "Removing" in
`crates/wakeshot-windows/src/agent_hooks.rs`, with these choices where the text above leaves room:

- Entry points: `at_start` (the tray, through `app::startup::add_agent_hooks`, on a thread of its own, which posts
  `Event::HooksAdded` per folder added; called by `tray_mode::start_agent_hooks` in `serve`), `on_uninstall`
  (from `RealHookEnv::remove_agent_hooks`), `remove_command` (for the CLI's `hook <agent> remove`, in `hook.rs`).
  Antigravity adds one entry to `AGENTS`: its folders, fixed files, a `present` check (`agy.exe`) and a `contents`
  function that refuses a path outside its allow list.
- "Installed": the tray exe's folder is `%LocalAppData%\HaikAsatryan.Wakeshot\current` (ASCII case ignored); the hook
  files name `wakeshot.exe` in it, and nothing is added when that file is missing.
- A fixed file counts as Wakeshot's when it hashes to the marker's value or holds what this version writes, so a stop
  between two writes of a rewrite (files first, the marker last, each with `store::write_atomic`) never turns the
  folder into "changed by someone". A `write_atomic` temp file of a fixed name is a leftover, deleted, not an extra.
- A staging folder also counts as Wakeshot's when a file holds only the start of this version's bytes, or the marker
  is missing (a stop during the write).
- The reparse check reads each part with `symlink_metadata` and refuses any reparse point (junctions, links, cloud
  files); files are opened with `FILE_FLAG_OPEN_REPARSE_POINT` and refused if they turn out to be one.
- Uninstall: the marker stays while a fixed file stays; empty fixed subfolders go anyway. The rename to
  `wakeshot-removed` (or `-2` to `-9`) never replaces a folder of that name. A file held open without delete sharing
  fails the delete (32), and the folder's rename then fails too (5): one log line says to delete it by hand (tested).
- A staging folder that does not list or read is one log line and stays; the main folder's work goes on (review m3).
  `remove` is private: only `on_uninstall` and `remove_command`, which check the level, call it (m6).
- An elevated uninstall starts a copy only for an agent whose `local` folder exists, and logs each copy's outcome; the
  copy's own lines go to the `cli` log.
- A record that does not read as version 1 (another version, a value but `added` or `removed`, not JSON) adds nothing
  and is never rewritten: delete it to start over.
- Not built: "Source turned off" (no version drops a source yet).

## Failure cases

| Case | What happens |
|---|---|
| Tray not running | The hook still writes and deletes its files. A tray that starts later sees the working chats. |
| Automatic off | Files are written; the tray ignores agent sessions, as for every source. |
| Wakeshot removed without cleanup (data folder deleted by hand, uninstall hook killed, install folder deleted) | Each event runs PowerShell, the exe is not found, exit 127: Cursor counts the hook failed and goes on (fail open; verified in code, CLI and editor). Cost: one PowerShell start per prompt and per stop. Fix: delete `.cursor\plugins\local\wakeshot`. Whether Cursor shows the failure to the user: **not verified**. |
| Hook slow or stuck | Cursor waits for `beforeSubmitPrompt` before it sends the prompt (editor `await`, verified), so the hook delays each prompt by PowerShell's start plus the exe. `timeout` 5 s caps it. The exe never waits on the tray, the network or a user. |
| `stop` never comes (crash, kill) | The file stays; the hold lasts until the named process ends, or the chat's next `stop` or `sessionEnd`. When that process is long-lived and shared (Cursor's agent host daemon, which may outlive a window: not verified; Antigravity's IDE hub, one `agy.exe` for all IDE chats; its Remote Control daemon, which runs from logon), a missed end holds for the whole session on AC. The walk takes the nearest match, which is the process that ran the hook, so it never picks a longer-lived one above it. Live test rows: Esc and a crash in each. Any fallback beyond this is a time cap, which the product rule forbids: product question 5. |
| An agent elevated as another account (over-the-shoulder UAC) | The Medium copy runs as the Explorer user, so the file lands in that user's data folder and the console user's tray holds. It holds only ids; accepted. |
| Esc or the stop button | `stop` with `aborted` (CLI code: three `stop` calls; editor: one call with the status). **Not run.** |
| Cursor CLI interactive mode | Plugin-only `beforeSubmitPrompt` and `stop` are not fired (code read): the CLI then shows nothing. If the user has their own `~/.cursor/hooks.json` with those events, the gate opens and ours run too. Product question 1. |
| A newer Cursor changes the hook format | File `version` 2 that drops version 1: the hook never runs, so no hold (fails safe, no notice). Renamed input fields: `problem.json`, the "update Wakeshot" notice. Renamed events: no hold. The live test is repeated on each Cursor version Wakeshot claims. |
| Enterprise policy | "Allow Local Plugin Imports" off (the Enterprise default): the plugin is not loaded, no hold, nothing breaks. |
| Two Wakeshot installs | Velopack installs per user, so one per user, each with its own profile and `.cursor`. A dev tray beside the installed one never touches agent folders. |
| Two users on one PC | Each user's Wakeshot adds to that user's `.cursor` only. |
| Per-machine Cursor (system setup) | Plugins still come from the user's `.cursor`, so the same. |
| Portable Cursor | Whether it moves `.cursor`: **not verified**. If it does, Wakeshot finds nothing and adds nothing. |
| A marketplace plugin named `wakeshot` | Cursor's docs: a marketplace plugin wins over a local one with the same name, so ours would not run. Not likely; noted. |
| Cloud agents | Cursor runs no user-level hooks for them (docs); a cloud run shown in the editor may still fire the editor's hooks: **not verified**. |
| `HOME` set to another folder | The CLI looks for plugins under `userHomeDir`, else `HOME` (code); Wakeshot writes under the profile folder. Which home the CLI passes: **not verified**. |

## Privacy

- On stdin the hook gets Cursor's whole input: for `beforeSubmitPrompt` the prompt text and attached file paths; in
  every event `user_email`, `workspace_roots`, `transcript_path`, the model; for `sessionEnd` an error message.
- Rule: the hook keeps none of it. It parses four fields (above); the parser skips the rest without storing it. Only
  `conversation_id`, `generation_id`, the agent pid and start, and the label reach a file. Log lines hold fixed text,
  the event name, error codes and, on a problem, `cursor_version` cut to 32 characters of `0-9 A-Z a-z . -`.
  Never the input bytes. A test feeds a marker word as the prompt and as the value of each of the four fields (also
  as a number or an object, so the parse fails) and checks that no file and no log line holds it.
- Cursor itself, because a hook exists, hands the whole input (with the prompt and `user_email`) to the hook in one
  of two forms (CLI `619.index.js`, `executeCommandScript`): a temp file `%TEMP%\cursor-hooks-<random>\payload.json`,
  written with `wx` and deleted after the run (the CLI's form, verified), or a here-string in the PowerShell command
  line (`argv_heredoc`). Process-creation logging (Sysmon event 1, Defender for Endpoint, Security event 4688 with
  command-line auditing, common on company PCs) records full command lines and ships them to a central log. So the
  command-line form would put every prompt and the user's email into such logs, from the day Wakeshot adds its
  plugin, against the rule that prompts are never stored or logged (`product.md`).
- **Gate:** Wakeshot adds a hook only for an agent whose every app that loads the folder passes the input through a
  temp file or stdin. The Cursor editor and the Cursor CLI load one folder, so **if the editor uses the command line,
  Wakeshot adds no Cursor plugin at all**, for either app, until Haik decides (product question 4; default: none).
  That costs little: the CLI's interactive loop does not fire plugin-only prompt and stop hooks anyway (question 1).
  Antigravity: how `agy` passes the input is unknown; the same check is the first step of its build card.
- **How the build checks it.** The form is a runtime option, not fixed by version: in the CLI,
  `commandHookPayloadTransport` is a parameter its callers pass (`3026.index.js`, `4110.index.js`, `697.index.js` from
  a function, `2855.index.js` from a variable; verify review). So two checks:
  1. Static, first step of the Cursor build card, and again for each Cursor version Wakeshot claims: in the editor's
     code (`extensionHostProcess.js`, the agent host daemon) find every place the option is set, and say whether it is
     a constant, a user setting or an experiment flag. Anything but "temp file in every path": no Cursor plugin.
  2. Runtime, in every hook run, for versions the user updates to: before any write, the hook reads its parent
     PowerShell's command line (`NtQueryInformationProcess`, `ProcessCommandLineInformation`, which needs only
     `PROCESS_QUERY_LIMITED_INFORMATION`: **not verified**) and looks for the payload's own key (`"conversation_id"`;
     Antigravity `"conversationId"`). It keeps nothing of that text and logs none of it. Found: the hook writes no
     state, removes the agent's plugin by the fixed names (at Medium, step 4), and writes
     `agents\<agent>\blocked.json` (`{ "version": 1, "pid": ..., "procStart": ..., "problem": "command-line" }`),
     which the source reads like `problem.json`, so the "update Wakeshot" notice shows and the log line says why. At
     its next start the tray finds the plugin gone and records `removed` (the "user removed it" row), so it never
     adds it again. This limits the leak to the one prompt whose hook found it. If the command line cannot be read,
     the hook goes on: the static check stays the main gate.
- In the here-string form, Cursor doubles `'`, and `JSON.stringify` never writes a raw line break, so the payload is
  one line and cannot close the here-string. Whether PowerShell also ends a line at U+2028 or U+2029 (which
  `JSON.stringify` leaves raw): **not verified**. Moot while the gate holds.
- The README says it in the main text, not a fold (the lines below).

## Antigravity

It needs hooks: `research/antigravity-signals.md` (card `p4-antigravity-research`, still in progress when read on
2026-11-06; CLI 1.5.2 and IDE 2.6.0, docs, not run). The same pattern applies, with these differences:

| Point | Antigravity (from that research) | Design |
|---|---|---|
| Folder | `%USERPROFILE%\.gemini\config\plugins\wakeshot\` with `plugin.json` and `hooks.json` (not `hooks/hooks.json`); CLI and IDE read the same folder; a disabled plugin runs no hooks | The same marker, record, add, rewrite and removal rules; fixed files `plugin.json`, `hooks.json`, `wakeshot-hooks.json` in `wakeshot\` only. The global `~\.gemini\config\hooks.json` is the user's file, never edited |
| Presence | `.gemini` is also Gemini CLI's folder (dropped from v1) | The research's rule 1: `agy.exe` in `%LOCALAPPDATA%\agy\bin\` or `%USERPROFILE%\.gemini\bin\`, and `.gemini\config\` already exists. Wakeshot never creates `config\` itself (it may create `plugins\` in it). With `ANTIGRAVITY_GEMINI_DIR` set (meaning not verified): add nothing |
| File | Named hook groups: `{ "wakeshot": { "PreInvocation": [...], "Stop": [...] } }`; `timeout` in seconds, default 30 | `timeout` 5 |
| Events | `PreInvocation` before every model call; `Stop` when the loop ends, with `fullyIdle` (false: background tasks still run); no session end event, no approval event | `PreInvocation`: write the file with `"state": "working"` (skip the write when it already says so for this process). `Stop` with `fullyIdle` true: delete; false: rewrite it with `"state": "background"`, which the source reads as `Activity::Idle` with one open task, so it holds while `agy.exe` lives, like Claude's `shell` (the research's rule 4). Approval waits count as working |
| Session | `conversationId` (camelCase, a UUID) on stdin | File name, the same UUID rule, lowercased |
| Agent process | `agy.exe`: the CLI from `%LOCALAPPDATA%\agy\bin\`, the IDE's hub from `%USERPROFILE%\.gemini\bin\` (one hub for all IDE chats) | Walk to the first `agy.exe`; label by its folder ("Antigravity CLI", "Antigravity") |
| Shell | How Windows starts the command (direct, `cmd` or PowerShell) is not documented; how it passes the input (stdin, a file or the command line) neither | The same command, `wakeshot.exe hook antigravity <event>`, as a bare absolute path, only when every character of the path is in an allow list: `A-Z a-z 0-9 \ : . _ -` and letters above ASCII. Anything else (a space, a quote, `{ } [ ] = ! @ # & ( ) ; , ^ % $`, backtick) gets no Antigravity hooks (one log line) until the live check names the shell and its quoting. The input form is the Privacy gate's check. The research's idea of a copy of the exe inside the plugin folder is rejected: an exe in another app's folder outlives Wakeshot and needs a copy at each update |
| Cost | `PreInvocation` fires on every model call, so a turn with 30 steps runs the hook about 30 times | Measure in the live test; the exe stays under 50 ms |
| Output | Docs disagree on `Stop`'s `decision`; non-zero exits fail open | Print `{}`; the live test confirms a stop goes through |

The Antigravity build card first names how `agy` passes the input (the Privacy gate), then settles the shell and the
command form, and checks that a dropped folder loads without `agy plugin install`.

**Built** (`p4-antigravity-source`, 2026-11-06). The Privacy gate holds: `agy` 1.5.2 (the CLI, and the IDE 2.6.0's
hub, the same binary) runs each hook as `cmd /c <command>` through Go's `os/exec`, with the input on stdin and
`ANTIGRAVITY_CONVERSATION_ID` in the environment; the IDE's JS runs no hook (`research.md`, "Antigravity hook payload
form"). Where the build differs from or settles the table above:

- Shell and command: settled by code, not by a live check. Go wraps the command in quotes for `cmd`, which strips them
  again; a quote inside would become `\"`, which `cmd` does not read (checked with this PC's `cmd`). So the allow list
  stays as designed, and `~` stays out too (`agy` expands only a leading `~/`, but no known folder holds a `~`).
  Added by `p4-live-cursor`: a path the list refuses (a space in the profile path) uses its 8.3 form
  (`GetShortPathNameW`) when that passes the list with `~` allowed inside a name; a volume that makes no short names
  gives the long form back, so no hooks then, as before (tested on this PC's `C:`).
- Input: `fullyIdle` is always in a `Stop` input (protojson `EmitUnpopulated`, a plain bool), so a `Stop` without a
  plain `true` or `false` there is unreadable input: `problem.json`, the "update Wakeshot" notice, the chat's file
  left as it was. The input has no event name, turn id or version field, so none is checked or logged.
- Labels: "Antigravity CLI" for an `agy.exe` under `%LocalAppData%\agy\bin\`, "Antigravity" for any other (the IDE's
  hub in `.gemini\bin`, or an install the research did not name), so the hook never misses an `agy.exe` it could pair
  with; the walk takes the nearest, as for Cursor.
- `PreInvocation` prunes the folder (no session end event, so it is the only turn start); `Stop` does not.
- Files: `hooks.json` first, `plugin.json` last, then the marker: `agy`'s plugin listing has no dot-name check (code
  read; live: it loads them), so a staging folder becomes a plugin only with its last file. Since `p4-live-cursor`
  the staging folder sits in `.gemini\config\`, outside `plugins\`, so that is a second guard. `plugin.json` holds `name`, `version`,
  `description` and `author`, the fields `agy`'s manifest type reads.
- Presence: `ANTIGRAVITY_GEMINI_DIR` is read from the tray's environment, like the agents' variables in
  `tray_mode::sources`.
- Not checked by code: that a dropped folder loads without `agy plugin install` and is enabled by default (the
  enabled state is kept outside the folder now, in an `enabledPlugins` map), and that `{}` lets a `Stop` through
  (`agy`'s embedded doc: any `decision` but `continue` allows it). Live test 7.
- For the tray: `tray_mode::hooked_sources` starts both hooked sources, and `tray_mode::start_agent_hooks` makes the
  one `startup::add_agent_hooks` call in `serve`.

**Changed** (`p4-antigravity-source`, orchestrator decision 2026-11-06, for Haik's review, easy to undo): in "Adding",
the first row ("missing | any | any | nothing") now also forgets the agent's mark. An agent's own folder gone at a
start (`.cursor`; `.gemini` or `.gemini\config`) is the agent going away, not the user removing Wakeshot's hooks, so a
reinstall that makes the folder again gets the hooks again, with the notice. Only Wakeshot's folder gone, the agent's
still there, stays "the user removed it". A mark of either kind is forgotten, `removed` too: that "no" was given to
the agent that went. A reinstall between two tray starts is not seen (the folder is back by the next start), so its
hooks stay removed: change it when that shows up, for example by keeping the agent folder's creation time in the
record.

## README lines

For `p4-readme-agents`:

- Agents: "For Cursor and Antigravity, Wakeshot adds a small hook folder of its own (`.cursor\plugins\local\wakeshot`
  and `.gemini\config\plugins\wakeshot` in your user folder) and tells you once. Uninstalling Wakeshot removes them;
  delete a folder to remove it yourself, and Wakeshot leaves it removed."
- Prompts, in the main text, not a fold: "While a hook runs, the agent hands it your prompt, and in some modes writes
  it to a temp file or a command line first. Wakeshot's hook keeps none of it, and Wakeshot adds no hook to an agent
  that puts prompts on a command line."
- Uninstall: "Uninstall also removes the hooks Wakeshot added to your agents (Cursor, Antigravity). If you changed
  one, it is moved to `wakeshot-removed` (or `wakeshot-removed-2` and so on) in `.cursor` or `.gemini\config`; delete that
  folder if you do not need it."
- The notice gets no link: a notice click can only open the log or the taskbar settings (`NoticeClick`), and a
  link would be one more contract change for little gain; the README is where users look.

## Contract changes the build needs

Beyond `p4-contracts-r2` (which made `SourceId::Cursor` and `Antigravity`, the stubs `sources/cursor.rs`,
`sources/antigravity.rs` and `agent_hooks.rs`):

1. `wakeshot-cli`: `Command::Hook { agent: String, event: String, values: Vec<String> }` in the frozen
   `command.rs`, its line in `main.rs`, the parse in `args.rs` (`hook <agent> <event> [--checked <value>...]`, not in
   `help`; `values` empty without `--checked`), a stub `hook.rs`.
2. `wakeshot-windows::process`: the parent pid in the public list, `ProcessEntry.parent: u32` (the private `Listed`
   already reads it from `Reserved2`); `image_path(pid) -> Result<PathBuf, WinError>`
   (`QueryFullProcessImageNameW`); and `current_above_medium() -> Result<bool, WinError>` (this process's
   `TokenIntegrityLevel` above Medium; the same read exists privately in `process.rs` tests and `store.rs`); and
   `harden_dll_search()` (`SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32)`); and
   `command_line_contains(pid, needle: &str) -> Result<bool, WinError>` for the runtime transport check, which reads
   the command line, searches it and drops it, so no caller ever holds or logs it. The bins call Windows only through
   `wakeshot-windows`. `spawn_outside_tree` and `ProcessHandle::wait` exist.
3. `wakeshot-core`: `Event::HooksAdded { agent: &'static str }`, `Notice::HooksAdded { agent: &'static str }`, the
   core's answer (`Notify`, once per event), the words in `text.rs`.
4. `wakeshot-windows::hooks`: `HookEnv::remove_agent_hooks`, called in `on_uninstall` after `remove_notices`.
5. A stub `wakeshot-core/src/sources/hooked.rs` for what Cursor and Antigravity share (the state file format, the id
   rule, the event rules, the parent walk over a process list), so neither source module owns the other's code; and
   the `Owner:` line of `agent_hooks.rs` set to the install card if the build is split (card Result).
6. `wakeshot-windows::store`: `write_atomic`'s temp name gets a random part and opens with `CREATE_NEW`, not
   `CREATE_ALWAYS`; on `ERROR_FILE_EXISTS` it tries a new name (at most 3 times). No delete step, so the change adds
   no delete of a name an attacker picks. A temp file left by a crash then stays (it ends in `.tmp`, which no reader
   opens); the hook's prune removes its own. This changes every caller. What it closes: only the hard-link overwrite.
   A junction plus an object manager link still redirects the temp file's create, so an elevated `wakeshot run`, and
   the uninstall's own journal and data folder work under an elevated `winget uninstall` (not checked), keep that gap
   until they get the same Medium rule as the hooks (verify review). The hooks do not depend on it: they run at
   Medium.

No new `WantedFile`, `Liveness`, data file enum or mutex in `store::Names`. `Win32_System_Threading`,
`Win32_System_LibraryLoader` (for `SetDefaultDllDirectories`) and `Win32_UI_Shell` are on in
`crates/wakeshot-windows/Cargo.toml` (checked 2026-11-06); `NtQueryInformationProcess` needed one more `windows`
feature, `Wdk_System_Threading` (added by `p4-contracts-r3`; a feature is not in `Cargo.lock`).

**Built** (`p4-contracts-r3`, 2026-11-06). All six items, with these shapes where they differ from the list above:

- `image_path(process: ProcessRef) -> Result<Option<PathBuf>, WinError>` and `command_line_contains(process:
  ProcessRef, needle: &str) -> Result<bool, WinError>`: a `ProcessRef`, not a pid, so a pid reused since the list was
  read never answers for another process; `None` and `false` when that process no longer runs. The walk has each
  parent's start from the list.
- `HookEnv::remove_agent_hooks(&mut self)` returns nothing: it logs one line per agent itself, through the hook's
  log, and never fails the hook. Its `RealHookEnv` body is an empty stub for `p4-agent-hooks-install`.
- The `hook` parse: a missing word is empty; any shape but `hook <agent> <event>` and `hook <agent> <event>
  --checked <value>...` (one value at least) gives empty words; `--checked` and `hook` in any case. Never `Help` or
  `Bad`.
- `write_atomic`'s temp name is `<name>.<pid>.<n>.<16 hex>.tmp`; `store::temp_target(name)` gives `<name>` for that
  shape only, for the prune. Three names in all, then `ERROR_FILE_EXISTS`.
- Added beyond the list: `store::local_app_data()` (the known folder, for `%LocalAppData%\cursor-agent\`, since the
  CLI calls Windows only through `wakeshot-windows`), and `tests/hook.rs` in the CLI (owner `p4-cursor-source`).
- The CLI has no JSON crate of its own, so the input parse and the state file text go in `sources::hooked` (core,
  which has `serde`), as its module doc says; a `serde_json` line in `wakeshot-cli` would change `Cargo.lock`.

## Live test (for the live card)

0. Before any hook ships (the first step of the Cursor build card, by reading code, no install needed): which
   payload form the Cursor editor uses on Windows (temp file or command line), from its spawn code in
   `extensionHostProcess.js` and the agent host daemon, at every place the option is set (constant, setting or
   experiment flag). Command line anywhere: no Cursor plugin at all until Haik decides (product question 4). The same
   for `agy` in the Antigravity build card. Live: the runtime check reads a real hook's parent command line.
1. The editor loads the local plugin with no prompt; `beforeSubmitPrompt`, `stop` (completed, Esc, error) and
   `sessionEnd` each run the exe; the process found is a `Cursor.exe` that lives as long as the window; the
   `conversation_id` has the UUID shape.
2. The prompt hook and the stop hook carry the same `generation_id` for one turn.
3. The CLI: which of the three fire with only the plugin, in interactive and in `-p` mode.
4. Time from send to the agent's start with and without the hook (PowerShell's start; whether Cursor passes
   `-NoProfile`).
5. With the exe renamed away: what Cursor shows, and that prompts still go.
6. Uninstall removes the folder; a folder the user changed moves to `.cursor\wakeshot-removed`; whether the editor
   skips dot-named folders (only for leftovers of test builds now).
7. Antigravity: a dropped folder loads in the CLI and the IDE; dot-named folders are skipped; the shell and the
   hook's parent; the cost per `PreInvocation`; whether Esc fires `Stop`; `fullyIdle` with a dev server; `{}` lets a
   stop through; Esc and a crash in the IDE hub and in a Remote Control chat.
8. Each agent with the exe in a folder named with `ё`, `ւ` and a space (a test copy of the plugin pointing there).
9. An elevated agent: the Medium copy writes the file; a test under an elevated token with a junction in place of
   `agents\cursor` shows that nothing is written or deleted through it (the elevated runner exists on the laptop).

## Risks for the security review

| # | Risk | What the design does |
|---|---|---|
| 1 | Wakeshot writes into another app's folder, which runs commands from it | Only a plugin folder of its own name, never a user file; only the installed tray writes it; one notice; the README lists it; uninstall removes it |
| 2 | Command injection through the exe path in `hooks.json` | Single-quoted path, ASCII `'` doubled; a path with a curly single quote or a control character gets no hooks; arguments are fixed ASCII words; `serde_json` writes the JSON; non-ASCII names in the live test |
| 3 | A planted link redirects Wakeshot's writes or the uninstall's deletes | All agent-folder work at Medium only, where a swap gains nothing; no write or delete through a reparse point; deletes only the fixed names; never recursive |
| 4 | The hook blocks or changes the user's prompt | Always exit 0, always `{}`, also after a panic (a panic hook prints `{}` and exits 0); no path exits 2; never `continue`, `permission`, `followup_message` or `failClosed`; tests check each event's output and exit code |
| 5 | The hook slows every prompt | Four-field parse, one process list, one small file; target under 50 ms; `timeout` 5 s; never waits on the tray; hooks only turn start and end, never tool events |
| 6 | Prompt text, paths or the user's email leak into Wakeshot's files or log | Parse only four fields; skipped fields never stored; fixed log text; parse errors logged by class and position only, never `serde_json`'s text; a marker-word test over files and log, also as each field's value |
| 7 | Prompts and the email in a process command line, so in audit logs | Gate: a hook only for an agent whose every app that loads the folder uses a temp file or stdin; the Cursor editor's form is the first step of its build card (static check of every place the option is set); command line means no Cursor plugin at all, for either app, until Haik decides; a runtime check of the parent's command line removes the plugin after at most one prompt if a later version switches; the README says it in the main text |
| 8 | A bad `conversation_id` used as a file name (traversal, device names, case twins) | UUID shape only, lowercased; anything else is unreadable input |
| 9 | A forged state file keeps the PC awake | Any process of the same user can already do that through a hold file (a documented format). A forged file holds only while the pid and start it names run (the watch thread checks both); the battery floor still steps back. Its label maps to the source's fixed strings, so no file text reaches the tooltip. A forged `problem.json` can show a false "update Wakeshot" notice once per start: accepted |
| 10 | A reused pid holds forever | `procStart` must match (`Liveness::Process`); the walk refuses a parent that started after its child |
| 11 | A stuck "working" (no `stop`) | Ends with the nearest agent process, the chat's next `stop` or `sessionEnd`; the battery floor; the live test checks Esc and crashes, also in Antigravity's hub and Remote Control daemon; a time cap is Haik's call (question 5) |
| 12 | Hooks left after Wakeshot is gone | Fail open with exit 127 (code read); the uninstall removes them first, and moves a folder it cannot empty to `wakeshot-removed` in the agent's own folder; the README says what to delete; the command form stays frozen for every later version |
| 13 | Fighting the user | A removed folder is never added back; a changed file is never overwritten or deleted; Cursor's own plugin switch is respected by leaving the folder alone |
| 14 | A dev build or a second copy writes hooks naming a temp exe | Only a tray in Velopack's `current` folder writes; the path names `current`, which survives updates |
| 15 | Two hook runs race on one chat file | Named mutex for the read and write, atomic rename, the turn id check on `stop` |
| 16 | Enterprise rules | Cursor's admin switch for local plugins decides; Wakeshot never works around it |
| 17 | Antigravity's unknown shell | Bare absolute path only when every character is in an allow list; otherwise no hooks until a live check; never an exe copied into the agent's folder |
| 18 | Antigravity's `PreInvocation` runs on every model call | The exe skips the write when the file already says working for this process; the live test measures the cost per call |
| 19 | An elevated hook, tray or uninstall writes or deletes in a folder a Medium process can swap or hard-link | File work only at Medium: an elevated hook passes its checked values to a Medium copy (`spawn_outside_tree`), an elevated uninstall runs `hook <agent> remove` the same way, an elevated tray skips; a copy that is not Medium writes nothing; the copy re-checks every value and drops a "working" write past its deadline; temp files with `CREATE_NEW`; the prune deletes only version 1 files of gone processes and its own temp names; a junction test under an elevated token |
| 20 | A marker anyone can write steers the rewrite or the uninstall | The file set is fixed in code; the marker has fixed keys and hashes only, no paths; any extra key, file or folder means "changed by someone"; empty folders removed only up to `wakeshot\` |
| 21 | Gemini CLI's `.gemini` taken for Antigravity | Presence needs `agy.exe` in its install folder and an existing `.gemini\config\`; Wakeshot never creates `config\`; `ANTIGRAVITY_GEMINI_DIR` set means no hooks |
| 22 | DLL planting from the repo folder a `stop` hook runs in | `SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32)` first thing; no exploitable DLL known, not tested |
| 23 | A leftover staging folder loads with live hooks | Created with "fail if it exists" in the agent's own folder, outside the folder the agent loads plugins from; removed by the fixed names at each start and at uninstall |
