# Agent hooks

Design, 2026-11-06 (card `p4-hooks-design`). A security review reads it before any build. Later the top part
becomes the user reference, like `hold-files.md`.

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
- What it keeps: the chat's id, the turn's id and the Cursor process. Never the prompt, file names or your email.
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
  wakeshot-hooks.json           Wakeshot's marker: {"version":1,"wakeshot":"<version>","files":{"<relative path>":"<FNV-1a 64 of what Wakeshot wrote>"}}
```

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
- PowerShell single quotes expand nothing (`$`, backtick). Wakeshot refuses to add hooks when the path holds a
  single-quote character (`'`, U+2018, U+2019, U+201A, U+201B: PowerShell counts all five as quotes, from its
  language spec, not re-read today) or a control character: one log line, no hooks, no notice.
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

1. Stream stdin through `serde_json` into a struct of four fields only: `conversation_id`, `generation_id`,
   `hook_event_name` and `cursor_version`. Every other field (the prompt, attachments, email, paths) is skipped by
   the parser without being stored. At most 64 MiB is read, then stdin is drained to its end, so Cursor's pipe never
   breaks. Cursor sends UTF-8 (the CLI's temp-file form sets it; verified); a byte order mark is skipped; input that
   does not parse is unreadable (below).
2. Check the input: `conversation_id` must be 1 to 80 characters of `A-Z a-z 0-9 - _` (it becomes a file name);
   `generation_id` at most 80 of the same, or empty. `hook_event_name`, if present, must equal the event in the
   command; else the input counts as unreadable.
3. Find the agent process: list processes (`NtQuerySystemInformation`, as `process::list` does) and walk up from the
   hook's parent, at most 10 steps. Each parent must be listed and must have started at or before its child, so a
   reused pid ends the walk. Take the first `Cursor.exe` (editor, label "Cursor"), or the first `node.exe` or
   `cursor-agent.exe` whose full image path lies under `%LocalAppData%\cursor-agent\` (CLI, label "Cursor CLI";
   install layout from `research/other-agent-signals.md`). The path comes from `QueryFullProcessImageNameW` with
   `PROCESS_QUERY_LIMITED_INFORMATION`. No match: write nothing, one log line.
4. Under the mutex `Local\Wakeshot.AgentHooks` (wait at most 1 s, then go on without it): apply the event to
   `%LocalAppData%\Wakeshot\agents\cursor\<conversation_id>.json` (create the folders; `store::write_atomic`). On
   `beforeSubmitPrompt` also delete any file in that folder whose process is not in the list.
5. Print `{}` and exit 0. Always, also after any error. `{}` merges to nothing (per-field merge), so it never
   changes another hook's answer, and a non-empty JSON reply keeps Cursor from counting the hook as failed.

Unreadable input (step 1 or 2) with a found agent process writes `agents\cursor\problem.json` (below) instead. Every
failure is one log line in `logs\wakeshot.log` (process `cli`, area `agent-hook`) with fixed text, the event name
and an error code; never input bytes. Target: under 50 ms for the exe; PowerShell's own start is Cursor's cost.

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
- The tray never writes these files and never deletes them; the hook prunes them, and the uninstall deletes the data
  folder.

### How the source reads them

`sources::cursor` (core, `Owner: p4-cursor-source`):

- `wanted()`: one `WantedFile::Folder { dir: <data>\agents\cursor, accept: name ends in ".json" }` (temp names from
  `write_atomic` do not end in `.json`).
- `feed`: `Whole` parses version 1; a newer `version` is ignored with a log line; a bad file gets `RetryOnce`, then
  no signal; `Gone` drops it.
- `sessions()`: one `SessionFacts` per chat file: key `(Cursor, conversation_id)`, `Liveness::Process { pid, start }`,
  `Activity::Working` (for `state` `background`: `Activity::Idle` with `open_tasks` 1), no timers, the file's label,
  `FormatHealth::Ok`. `sources::antigravity` reads `agents\antigravity\` the same way. `problem.json` gives one
  `Activity::Idle` session with `FormatHealth::Unreadable("Cursor hook input")`, so the "update Wakeshot" notice
  shows once per start while that process lives.
- No new `WantedFile` or `Liveness` variant is needed.

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
| exists | ours, unchanged, older content | any | rewrite the changed files, one log line, no notice; record `added` |
| exists | ours, a file changed by someone | any | leave it all, one log line per start; record `added` |
| exists | not ours | any | never touch it, one log line |

- "Ours": `wakeshot-hooks.json` parses. "Unchanged": each listed file hashes to the marker's value. The hash finds
  edits, not attacks: a user who can edit the files can do anything as that user anyway.
- Add: build the whole folder as `plugins\local\.wakeshot-new-<pid>\` (Cursor skips dot names, verified for the CLI),
  then rename it to `wakeshot`. Cursor never sees half a plugin. Rewrite: each file with `store::write_atomic`.
- Never through a link: `.cursor`, `plugins`, `local` and `wakeshot` must be plain folders (no reparse point), else
  nothing is written, one log line.
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
  the restore: the hook files name an exe the uninstall deletes, the same reason the Run value goes first. For each
  agent folder that is ours: delete each listed file whose hash still matches, then the marker, then the folders
  that are now empty. A file someone changed or added stays, and so does its folder; one log line names it. Never a
  recursive delete, never through a link. The record goes with the data folder step.
- **Updated hook**: nothing. The command path does not change, and the next tray start rewrites older content.
- **Source turned off** (a later version that drops the Cursor source, or a switch if Haik wants one): the tray runs
  the same removal at start and records `removed`. Automatic off does not remove anything: it is a pause.
- **The `wakeshot hook` command form is frozen**: every later version accepts `hook <agent> <event>`, and an agent or
  event it does not know prints `{}` and exits 0, so hooks left by any version never fail.

## Failure cases

| Case | What happens |
|---|---|
| Tray not running | The hook still writes and deletes its files. A tray that starts later sees the working chats. |
| Automatic off | Files are written; the tray ignores agent sessions, as for every source. |
| Wakeshot removed without cleanup (data folder deleted by hand, uninstall hook killed, install folder deleted) | Each event runs PowerShell, the exe is not found, exit 127: Cursor counts the hook failed and goes on (fail open; verified in code, CLI and editor). Cost: one PowerShell start per prompt and per stop. Fix: delete `.cursor\plugins\local\wakeshot`. Whether Cursor shows the failure to the user: **not verified**. |
| Hook slow or stuck | Cursor waits for `beforeSubmitPrompt` before it sends the prompt (editor `await`, verified), so the hook delays each prompt by PowerShell's start plus the exe. `timeout` 5 s caps it. The exe never waits on the tray, the network or a user. |
| `stop` never comes (crash, kill) | The file stays; the hold lasts until the named Cursor process ends, or the chat's next `stop` or `sessionEnd`. |
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
  Never the input bytes. A test feeds a prompt with a marker word and checks that no file and no log line holds it.
- Cursor itself, because a hook exists, writes the whole input (with the prompt) to a temp file for the hook's run
  (CLI 2026.11.05), or puts it in the PowerShell command line in its other mode. That is Cursor's doing; any hook has
  it. The README does not hide it (the line below).

## Antigravity

It needs hooks: `research/antigravity-signals.md` (card `p4-antigravity-research`, still in progress when read on
2026-11-06; CLI 1.5.2 and IDE 2.6.0, docs, not run). The same pattern applies, with these differences:

| Point | Antigravity (from that research) | Design |
|---|---|---|
| Folder | `%USERPROFILE%\.gemini\config\plugins\wakeshot\` with `plugin.json` and `hooks.json` (not `hooks/hooks.json`); CLI and IDE read the same folder; a disabled plugin runs no hooks | The same marker, record, add, rewrite and removal rules; the global `~\.gemini\config\hooks.json` is the user's file, never edited |
| File | Named hook groups: `{ "wakeshot": { "PreInvocation": [...], "Stop": [...] } }`; `timeout` in seconds, default 30 | `timeout` 5 |
| Events | `PreInvocation` before every model call; `Stop` when the loop ends, with `fullyIdle` (false: background tasks still run); no session end event, no approval event | `PreInvocation`: write the file with `"state": "working"` (skip the write when it already says so for this process). `Stop` with `fullyIdle` true: delete; false: rewrite it with `"state": "background"`, which the source reads as `Activity::Idle` with one open task, so it holds while `agy.exe` lives, like Claude's `shell` (the research's rule 4). Approval waits count as working |
| Session | `conversationId` (camelCase) on stdin | File name, same character rule |
| Agent process | `agy.exe`: the CLI from `%LOCALAPPDATA%\agy\bin\`, the IDE's hub from `%USERPROFILE%\.gemini\bin\` (one hub for all IDE chats) | Walk to the first `agy.exe`; label by its folder ("Antigravity CLI", "Antigravity") |
| Shell | How Windows starts the command (direct, `cmd` or PowerShell) is not documented | The same command, `wakeshot.exe hook antigravity <event>`, as a bare absolute path: that works in all three when the path has no space and none of `& ( ) ; , ' " ^ % $` and backtick. A path with any of them gets no Antigravity hooks (one log line) until the live check names the shell and its quoting. The research's idea of a copy of the exe inside the plugin folder is rejected: an exe in another app's folder outlives Wakeshot and needs a copy at each update |
| Cost | `PreInvocation` fires on every model call, so a turn with 30 steps runs the hook about 30 times | Measure in the live test; the exe stays under 50 ms |
| Output | Docs disagree on `Stop`'s `decision`; non-zero exits fail open | Print `{}`; the live test confirms a stop goes through |

The Antigravity build card settles the shell and the command form first, and checks that a dropped folder loads
without `agy plugin install`.

## README lines

For `p4-readme-agents`:

- Agents: "For Cursor and Antigravity, Wakeshot adds a small hook folder of its own (`.cursor\plugins\local\wakeshot`
  and `.gemini\config\plugins\wakeshot` in your user folder) and tells you once. Uninstalling Wakeshot removes them;
  delete a folder to remove it yourself, and Wakeshot leaves it removed."
- Uninstall: "Uninstall also removes the hooks Wakeshot added to your agents (Cursor, Antigravity)."
- A folded detail may add: "While any hook runs, the agent hands it the prompt; Wakeshot's hook keeps none of it."

## Contract changes the build needs

Beyond `p4-contracts-r2` (which made `SourceId::Cursor` and `Antigravity`, the stubs `sources/cursor.rs`,
`sources/antigravity.rs` and `agent_hooks.rs`):

1. `wakeshot-cli`: `Command::Hook { agent: String, event: String }` in the frozen `command.rs`, its line in
   `main.rs`, the parse in `args.rs` (`hook <agent> <event>`, not in `help`), a stub `hook.rs`.
2. `wakeshot-windows::process`: the parent pid in the public list, `ProcessEntry.parent: u32` (the private `Listed`
   already reads it from `Reserved2`), and `image_path(pid) -> Result<PathBuf, WinError>`
   (`QueryFullProcessImageNameW`), since the bins call Windows only through `wakeshot-windows`.
3. `wakeshot-core`: `Event::HooksAdded { agent: &'static str }`, `Notice::HooksAdded { agent: &'static str }`, the
   core's answer (`Notify`, once per event), the words in `text.rs`.
4. `wakeshot-windows::hooks`: `HookEnv::remove_agent_hooks`, called in `on_uninstall` after `remove_notices`.
5. A stub `wakeshot-core/src/sources/hooked.rs` for what Cursor and Antigravity share (the state file format, the id
   rule, the event rules, the parent walk over a process list), so neither source module owns the other's code; and
   the `Owner:` line of `agent_hooks.rs` set to the install card if the build is split (card Result).

No new `WantedFile`, `Liveness`, data file enum, mutex in `store::Names`, `Cargo` change or `windows` feature
(`Win32_System_Threading` and `Win32_UI_Shell` are on).

## Live test (for the live card)

1. The editor loads the local plugin with no prompt; `beforeSubmitPrompt`, `stop` (completed, Esc, error) and
   `sessionEnd` each run the exe; the process found is a `Cursor.exe` that lives as long as the window.
2. The prompt hook and the stop hook carry the same `generation_id` for one turn.
3. The CLI: which of the three fire with only the plugin, in interactive and in `-p` mode.
4. Time from send to the agent's start with and without the hook (PowerShell's start; whether Cursor passes
   `-NoProfile`).
5. With the exe renamed away: what Cursor shows, and that prompts still go.
6. Uninstall removes the folder; a folder the user changed stays.
7. Antigravity: a dropped folder loads in the CLI and the IDE; the shell and the hook's parent; the cost per
   `PreInvocation`; whether Esc fires `Stop`; `fullyIdle` with a dev server; `{}` lets a stop through.

## Risks for the security review

| # | Risk | What the design does |
|---|---|---|
| 1 | Wakeshot writes into another app's folder, which runs commands from it | Only a plugin folder of its own name, never a user file; only the installed tray writes it; one notice; the README lists it; uninstall removes it |
| 2 | Command injection through the exe path in `hooks.json` | Single-quoted path; a path with any single-quote character or control character gets no hooks; arguments are fixed ASCII words; `serde_json` writes the JSON |
| 3 | A planted link redirects Wakeshot's writes or the uninstall's deletes | No write or delete through a reparse point; deletes only listed files by name and hash; never recursive |
| 4 | The hook blocks or changes the user's prompt | Always exit 0, always `{}`; never `continue`, `permission`, `followup_message` or `failClosed`; tests check each event's output and exit code |
| 5 | The hook slows every prompt | Four-field parse, one process list, one small file; target under 50 ms; `timeout` 5 s; never waits on the tray; hooks only turn start and end, never tool events |
| 6 | Prompt text, paths or the user's email leak into Wakeshot's files or log | Parse only four fields; skipped fields never stored; fixed log text; a marker-word test over files and log |
| 7 | Prompt in Cursor's temp file or PowerShell command line | Cursor's own behavior for any hook; noted in the README detail; not changeable by Wakeshot |
| 8 | A bad `conversation_id` used as a file name (path traversal) | Only `A-Z a-z 0-9 - _`, 1 to 80 characters; anything else is unreadable input |
| 9 | A forged state file keeps the PC awake | Any process of the same user can already do that through a hold file (a documented format). A forged file holds only while the pid and start it names run (the watch thread checks both); the battery floor still steps back |
| 10 | A reused pid holds forever | `procStart` must match (`Liveness::Process`); the walk refuses a parent that started after its child |
| 11 | A stuck "working" (no `stop`) | Ends with the Cursor process, the chat's next `stop` or `sessionEnd`; the battery floor; the live test checks Esc and errors |
| 12 | Hooks left after Wakeshot is gone | Fail open with exit 127 (code read); the uninstall removes them first; the README says how to delete them; the command form stays frozen for every later version |
| 13 | Fighting the user | A removed folder is never added back; a changed file is never overwritten or deleted; Cursor's own plugin switch is respected by leaving the folder alone |
| 14 | A dev build or a second copy writes hooks naming a temp exe | Only a tray in Velopack's `current` folder writes; the path names `current`, which survives updates |
| 15 | Two hook runs race on one chat file | Named mutex for the read and write, atomic rename, the turn id check on `stop` |
| 16 | Enterprise rules | Cursor's admin switch for local plugins decides; Wakeshot never works around it |
| 17 | Antigravity's unknown shell | Bare absolute path only when it holds no space or shell character; otherwise no hooks until a live check; never an exe copied into the agent's folder |
| 18 | Antigravity's `PreInvocation` runs on every model call | The exe skips the write when the file already says working for this process; the live test measures the cost per call |
