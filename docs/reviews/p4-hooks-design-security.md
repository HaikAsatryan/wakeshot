# Security review: agent hooks design

Review of `docs/agent-hooks.md` (design of 2026-11-06, card `p4-hooks-design`), before any build. The reviewer did
not write it. Read: the design, the card (Result, Questions), `product.md` "What keeps the PC awake" and UX,
`research/cursor-gemini-signals.md`, `research/antigravity-signals.md`, `research.md` "Cursor hooks from a local
plugin", `hooks.rs` (`on_uninstall`), `process.rs` (the list, `Listed.parent`, `spawn_outside_tree`), `store.rs`
(`write_atomic`), `hold-files.md`, the root `Cargo.toml` release profile. Nothing installed or run; no git.

Checked myself, as data: Cursor CLI `2026.11.05-86047df` (`agent-cli-package.zip`, Windows x64), `619.index.js`
`executeCommandScript` and `index.js`; Antigravity `docs/hooks` page (read 2026-11-06). Downloads deleted at the end.

Counts: 0 blocker, 3 major, 10 minor.

## Major

### 1. An elevated hook writes and deletes in a folder any Medium process controls

`agent-hooks.md:120-128` (steps 3 and 4), `store.rs:217-262` (`write_temp`, `create_temp` with `CREATE_ALWAYS`,
`create_dir_all`).

The hook runs with the agent's token. Agents often run elevated (this laptop: every agent shell is High, `CLAUDE.md`;
many users run Cursor CLI or `agy` from an admin terminal). The hook then writes, and on `beforeSubmitPrompt` deletes,
in `%LocalAppData%\Wakeshot\agents\<agent>\`. That folder inherits the profile ACL, so any Medium process of the same
user can delete it and put a junction (or a junction plus an object manager link) in its place, or plant files in it.

Failure, step by step (not run; the techniques are the known public ones):

1. Malware at Medium reads `agents\cursor\<id>.json` (readable) to learn a chat id.
2. Delete path: it replaces `agents\cursor` with a junction to a folder only admins can write (say a vendor folder in
   `C:\ProgramData`). The next elevated `beforeSubmitPrompt` runs "delete any file in that folder whose process is
   not in the list" there. The design does not say whether a file that does not parse counts as "no process"; if it
   does, this is an elevated delete of every file in a folder the attacker picks.
3. Overwrite path: it plants hard links `<id>.json.<pid>.0.tmp` (one per likely pid) to a file only admins can write.
   `create_temp` opens with `CREATE_ALWAYS`, so the elevated hook truncates and overwrites that file with the state
   JSON. Hard links to a file the caller cannot write: allowed for a normal Medium caller through the native call
   (public research; not tested here).

So Wakeshot adds a Medium-to-High write and delete primitive that does not exist without it. That goes against
"never lower security". The same applies, with a smaller window, to a tray started from an elevated shell (it writes
into `.cursor\plugins\local\`), and to finding 2.

Fix (pick in the design, before the build):

- Prune only in a hook whose token is not elevated (`TokenElevation`), or move the prune to the tray, which runs at
  Medium. An elevated hook touches only its own chat's file.
- Lean option: an elevated hook does the parse and the walk, then hands the write to a Medium copy through the existing
  `process::spawn_outside_tree` (`process.rs:244-259`: an elevated caller gets a Medium child), with the four checked
  values as arguments. Or, simplest: an elevated hook writes nothing, one log line (cost: no hold for elevated agents,
  which is most of Haik's own use).
- Whatever runs: the temp file with `CREATE_NEW`, not `CREATE_ALWAYS`; the prune deletes only files that parse as
  version 1 and name a gone process, plus `.tmp` files of its own name shape.
- The tray and the uninstall touch no agent folder when their token is elevated (one log line).
- A test under an elevated token with a junction in place of `agents\cursor` (the elevated runner exists here).

### 2. The uninstall and the rewrite trust paths and hashes from a file anyone can write

`agent-hooks.md:60` (marker `"files":{"<relative path>":"<hash>"}`), `:204-205` ("Ours": the marker parses),
`:200` (rewrite "the changed files"), `:223-227` (uninstall deletes "each listed file whose hash still matches").

"Ours" means only that `wakeshot-hooks.json` parses. The file list and the hashes come from that file. Nothing says a
listed path must stay inside the plugin folder. FNV-1a 64 is not a secret: anyone who can read a file can compute it.

Failure: a Medium process writes a marker listing `..\..\..\..\..\..\..\Windows\System32\drivers\etc\hosts` with that
file's FNV-1a hash. The user runs `winget uninstall` from an admin terminal (common), so the Velopack uninstall hook
runs elevated (a child keeps its parent's token; not tested here). `remove_agent_hooks` checks the hash, it matches,
and it deletes the file. At Medium the same marker makes the uninstall delete any of the user's own files: it breaks
"never touch a file Wakeshot did not write" even with no privilege gain. The rewrite row has the same gap if it writes
by listed names.

Also not covered: an added file. The table only hashes listed files, so a `rules\`, `skills\` or `mcp.json` that
another program drops into `wakeshot\` loads in Cursor under Wakeshot's name, and the tray still calls the folder
"ours, unchanged".

Fix:

- The file list is a constant in Wakeshot's code (`.cursor-plugin\plugin.json`, `hooks\hooks.json`, the marker; for
  Antigravity `plugin.json`, `hooks.json`). The marker may carry hashes for those names only; any other key makes the
  folder "not ours".
- Deletes and rewrites only by those fixed names, each part checked as not a reparse point. Empty-folder removal only
  for `hooks\`, `.cursor-plugin\` and `wakeshot\` itself, never `local\` or `plugins\`.
- Any file or folder in `wakeshot\` beyond the fixed set counts as "changed by someone" (log, never rewrite).
- With finding 1: no agent-folder work under an elevated token.

### 3. The Cursor editor may put every prompt on a process command line, and Wakeshot turns that on by itself

`agent-hooks.md:46`, `:263-265`, risk row 7 (`:339`), README detail `:294`.

Checked in the CLI code (`619.index.js`, `executeCommandScript`): two transports. `windows_temp_file` writes the
payload to `%TEMP%\cursor-hooks-<random>\payload.json` (`flag: "wx"`) and deletes the folder in `finally`. The other,
`argv_heredoc`, builds `@'\n<payload with ' doubled>\n'@ | & <command>` as the command text, so the payload, with the
prompt and `user_email`, goes into the PowerShell command. The CLI picks the temp file (`commandHookPayloadTransport`
from a function that returns `"stdin"` for `true`, which maps to `windows_temp_file`). Which one the editor uses: not
verified (the design says so too).

Failure if the editor uses `argv_heredoc`: from the day Wakeshot adds its plugin, every Cursor prompt and the user's
email are in a process command line. Process-creation logging records full command lines (Sysmon event 1, Defender for
Endpoint, Security event 4688 with command-line auditing, all common on company PCs) and ships them to a central log.
The product rule is that prompts are never stored or logged (`product.md:32`, and this design's own privacy rule). It
is Cursor's code, but without Wakeshot's hook the user had no hook and no such log. The user is told only in a
"folded detail" that "may" be added.

Injection: in the here-string form the doubled `'` keeps a line from starting with `'@`. Whether the JSON can hold a
raw line break (which would let a curly quote U+2018 to U+201B close the here-string, since PowerShell counts those as
quotes) was not traced. Mark it unverified, but it is worth one look while reading the editor's code.

Fix:

- The Cursor build card first reads the editor's spawn code (`extensionHostProcess.js`, the agent host daemon) and
  names the transport. Gate: if it is the command line, the editor hook does not ship until Haik decides, knowing that
  prompts go into command lines and audit logs.
- The README says it in the main text, not a fold: "While a hook runs, Cursor hands it your prompt, and in some modes
  writes it to a temp file or a command line." Also the notice could link to that README part.
- Antigravity: how `agy` passes stdin is unknown; the same check in its build card.

## Minor

### 4. "Always `{}` and exit 0" is not true for a panic

`agent-hooks.md:129-130`; root `Cargo.toml:35` `panic = "abort"`.

With abort, `catch_unwind` cannot catch a panic, so a panic ends the process with a crash code and empty stdout. It
still fails open (not exit 2), so no prompt is blocked, but the design's promise and its tests ("`{}` and exit 0 for
every input") will not hold. Fix: the `hook` command sets a panic hook first that prints `{}`, flushes and calls
`process::exit(0)`, and logs no input. State that no path may exit 2 (Cursor's block code).

### 5. `serde_json` error text can quote input values

`agent-hooks.md:132-134`, `:259-262`.

A type error on one of the four fields reads like `invalid type: integer 123, expected a string`, with the value. If
a later Cursor puts text into a field Wakeshot reads (or a crafted input does), a log line that prints the error holds
input bytes. Fix: log only `Error::classify()` and line and column, never `Display`; the marker-word test also feeds
the marker word as the value of each of the four fields.

### 6. The chat id rule lets device names and case twins through

`agent-hooks.md:117-119`, risk row 8.

`CON`, `NUL`, `AUX`, `PRN`, `COM1`, `LPT1` pass `A-Z a-z 0-9 - _`. `CON.json.<pid>.0.tmp` can open the device on
Windows builds that treat a reserved base name with an extension as the device (Microsoft's naming docs say to avoid
them); `COM1` could open a serial port. `ABC` and `abc` are one file on NTFS. Only a crafted input does this, so the
impact is small. Fix: both agents use UUIDs (`conversationId (UUID)`, `research/antigravity-signals.md:137`; Cursor's
ids look the same, not verified), so require the UUID shape and lowercase it; or prefix every name (`c-<id>.json`).

### 7. Antigravity presence: `.gemini` is also Gemini CLI's folder

`agent-hooks.md:185-188` (presence is a folder), `:274` ("the same ... rules").

A Gemini CLI user (dropped from v1) has `%USERPROFILE%\.gemini\`. If presence means "`.gemini` exists", Wakeshot
creates `.gemini\config\plugins\wakeshot\` there and shows "Wakeshot now works with Antigravity", which is false.
Fix: use the research's rule 1 (`antigravity-signals.md:297`: `agy.exe` in `%LOCALAPPDATA%\agy\bin\` or
`.gemini\bin\`), and never create `config\` itself. `ANTIGRAVITY_GEMINI_DIR` moves the folder (meaning not verified):
then add nothing.

### 8. Path rules: Antigravity's is a deny list; non-ASCII names are not covered

`agent-hooks.md:81-83`, `:279`, risk rows 2 and 17.

- The Antigravity list (no space, none of `& ( ) ; , ' " ^ % $` and backtick) misses PowerShell and cmd specials:
  `{ }` end a bare word in PowerShell, `[ ]` act as wildcards in PowerShell command lookup, `=` splits the command
  name in cmd, `!` matters with delayed expansion, `@` and `#` at a token start. Windows user names cannot hold most of
  these, but a moved `%LocalAppData%` can. The result is "not found" (fails open), not injection, since the path is not
  an attacker's. Fix: an allow list (`A-Z a-z 0-9 \ : . _ -`, plus letters above ASCII), everything else gets no hooks.
- Non-ASCII user names (Armenian, Russian) are not named anywhere. If any step on the way decodes the command as a
  legacy code page (a BOM-less `.ps1` or `.bat`), `ё` (UTF-8 `D1 91`) turns into U+2018 and `ւ` (`D6 82`, in names
  like Արթուր) into U+201A in both cp1251 and cp1252, and PowerShell reads both as quotes: the command breaks. The
  check runs on the real path, so it cannot see this. Cursor and Go both start processes with UTF-16 command lines, so
  likely fine; how Cursor's shell executor hands over the command text was not traced. Fix: a live test row with the
  exe in a folder named with `ё` and `ւ` (and a space) for each agent.
- Names with `'` (O'Brien) get no Cursor hooks. Cursor's own parser accepts `''` inside a quoted path (`index.js`,
  the path scan skips doubled `'`), and PowerShell reads `''` as one quote. Optional: double the quote instead of
  refusing.

### 9. The staging folder can be left behind, and the editor may load it

`agent-hooks.md:206-207`.

A crash between create and rename leaves `plugins\local\.wakeshot-new-<pid>\` with live hooks. The CLI skips dot
names (verified); the editor: not verified. A leftover the editor loads keeps running a hook naming Wakeshot's exe
after uninstall, which removes only `wakeshot\`. Fix: create the staging folder with "fail if it exists"; at each start
and at uninstall remove `.wakeshot-new-*` folders that hold only the fixed files with matching content; add "the
editor skips dot names" to the live test.

### 10. After uninstall, a changed or locked file leaves a live plugin that names a deleted exe

`agent-hooks.md:223-227`, failure row "Wakeshot removed without cleanup".

If the user edited `hooks.json`, or Cursor holds a file open during the uninstall, the plugin stays and every prompt
starts PowerShell for a missing exe (exit 127, fails open; verified in the CLI code). The one log line goes away with
the data folder after a good restore, so nobody learns of it. The research's "retry later" (`antigravity-signals.md:305`)
is impossible after uninstall. Fix: when a fixed file cannot be deleted, rename `wakeshot\` to a dot name
(`.wakeshot-removed`), which the CLI skips and which keeps the user's edits; for Antigravity, check in the live test
whether a dot name is skipped. The README uninstall line says what to delete if anything is left.

### 11. Long-lived shared agent processes turn a missed end event into a session-long hold

`agent-hooks.md:49` (Cursor agent host daemon), `:242`, `:278`; `antigravity-signals.md:209-218`.

The Antigravity IDE hub is one `agy.exe` for all IDE chats and lives as long as the IDE; the Remote Control daemon is
an `agy.exe` that runs from logon; Cursor's agent host daemon may outlive a window (not verified). Antigravity has no
session end event and whether Esc fires `Stop` is unknown. So a cancelled turn holds until that process ends, which
may be the whole session on AC. Not a security hole, but it is item 5 of the brief. Fix: live test rows for Esc and a
crash in the IDE hub and in a Remote Control chat; say in the design what the walk takes when both a window process
and a daemon are ancestors. Any fallback beyond that is a time cap, which the product rule forbids by default, so it
is Haik's call.

### 12. The file's `label` reaches the tray as is

`agent-hooks.md:151`, `:164` ("the file's label").

A forged file can put any text in the tooltip and menu. Hold files already allow a 60-character `reason`, cleaned of
control characters (`hold-files.md:47`), so this is no worse if the same rule applies, but the design does not say.
Fix: the source maps the file's label to its own fixed strings (`Cursor`, `Cursor CLI`, `Antigravity`,
`Antigravity CLI`) and ignores any other value. A forged `problem.json` can show a false "update Wakeshot" notice
once per start; accept it, note it in the risk table.

### 13. Small hardening and doc gaps

- `stop` hooks run with the workspace root as working folder (`agent-hooks.md:48`), which is an untrusted repo. Call
  `SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32)` first thing in `hook`, so no DLL loads from that folder.
  No exploitable DLL is known; not tested.
- `docs/architecture.md:1455-1456` puts the parent walk in `agent_hooks` (Windows); `agent-hooks.md:309-311` puts it
  in `sources/hooked.rs` (core). Pick one.
- `product.md:258` still says "nothing that touches other apps"; the hooks rule (`:70-73`) is an exception to it. Add
  the exception there so the two lines agree.
- `research/cursor-gemini-signals.md:225-269` still describes a separate `wakeshot-hook` exe, opt-in per agent and
  `main.log` first; the design card already names this as stale.

## The design's risk table, row by row

| # | Real? | Answer holds? | Note |
|---|---|---|---|
| 1 | Yes | Partly | Fine at Medium. Elevated runs: finding 1. Added files: finding 2 |
| 2 | Yes | Yes for Cursor | Single quotes plus the five quote characters cover PowerShell. Antigravity's rule and non-ASCII: finding 8 |
| 3 | Yes | No | Link checks are fine at Medium, where a race gains nothing. The marker's own paths and elevated runs: findings 1, 2 |
| 4 | Yes | Mostly | `{}` merges to nothing (CLI code); Antigravity `Stop` allows any `decision` but `continue` (docs). Panic: finding 4 |
| 5 | Yes | Yes | One list, one small file, 5 s cap. Cost per prompt is PowerShell's start; the live test measures it |
| 6 | Yes | Mostly | Parse of four fields is right. `serde_json` error text: finding 5 |
| 7 | Yes | No | Not only privacy: command lines go to audit logs. Finding 3 |
| 8 | Yes | Mostly | No traversal possible. Device names and case: finding 6 |
| 9 | Yes | Yes | Same reach as a hold file. Label: finding 12 |
| 10 | Yes | Yes | Start time check stops reuse. A spoofed parent (`PROC_THREAD_ATTRIBUTE_PARENT_PROCESS`) gives only row 9's reach |
| 11 | Yes | Partly | Shared long-lived processes: finding 11 |
| 12 | Yes | Partly | Exit 127 append verified in CLI `index.js`. What stays after a failed delete: finding 10 |
| 13 | Yes | Yes | A removed folder stays removed |
| 14 | Yes | Yes | Path names `current`, which survives updates |
| 15 | Yes | Yes | Mutex, atomic rename, turn id |
| 16 | Yes | Yes | |
| 17 | Yes | Partly | Deny list: finding 8 |
| 18 | Yes | Yes | Skip-write when already working is right |

Not in the table and should be: an elevated agent or tray (finding 1), a marker anyone can write (finding 2),
prompts in audit logs (finding 3), Gemini CLI's `.gemini` (finding 7).

## Checked and fine

- No `continue`, `permission`, `followup_message`, `failClosed`, `matcher`: matches the CLI code, where a thrown
  error or a timeout turns into a block only with `failClosed: true` (exit 2 blocks on its own).
- The CLI's temp-file payload is deleted after each run (`finally`, `rm` recursive) and written with `wx`.
- Antigravity `-p`: the docs tie a non-zero `-p` exit to a hook's `errorMessage` output, not to its exit code, so
  `{}` does not change a headless run.
- Two users, per-user installs, dev trays, Store builds: the design's answers hold.
- The data folder step at uninstall already removes `agents\` (`hooks.rs`, `clean_data_folder`), and Rust's
  `remove_dir_all` does not follow links.

## Found outside scope

- `wakeshot run` in an elevated shell writes its hold file through the same `write_atomic` (`CREATE_ALWAYS` temp) in
  `holds\`, a Medium-writable folder: the overwrite half of finding 1 applies to it today. Not checked in detail.

## Build go

The build may start once findings 1 and 2 are written into the design (they change the hook's steps and the
uninstall step) and finding 3's check is the first step of the Cursor build card, with Haik's call before the editor
hook ships if the editor uses the command line. Minors can go into the build cards.
