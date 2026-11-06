# Live harness: agents run live tests without Haik

Phase: 4. Status: blocked (Part A done 2026-11-06 14:13; waits for Haik's sign-in sitting, "Steps for Haik", and
the orchestrator's go for Part B). Needs Haik: yes, once, after Part A: one short sign-in sitting with the
exact steps this card writes (only for logins that cannot be reused).
Agent: `wakeshot-worker`. Size: about 180K tokens (Part A about 140K, Part B about 40K).
Runs after: `p4-live-claude` (done). Part B after `p4-contracts-r2` (the orchestrator says go).
Parallel with: `p4-contracts-r2` (Part A builds only outside the workspace), `p4-antigravity-research`,
`p4-hooks-design`.
Build folder: `<scratchpad>\p4-live-harness\` (`target\` for cargo). Downloads in their own empty folder, as data.
Real laptop: installs for Haik's user only (no admin); no power settings; no screen tests.

## Goal

Haik wants minimum time at the laptop but real tests. Make every v1 agent testable live by agents: the agents
installed for his user, a driver that types into a terminal agent the way a person does, the watch harness for every
source, and a list of the logins that still need him, done in one short sitting.

## Read first

- `CLAUDE.md` (elevation note, cargo path, Test laptop), `docs/plan.md` "How we work".
- `docs/tasks/p4-live-claude.md` (the harness `crates/wakeshot-windows/examples/agent-watch.rs` and how it ran).
- `docs/tasks/p4-live-copilot.md` (Round 2 notes) and `docs/tasks/p4-live-codex.md`: what those tests must show.
- Research per agent, only the parts about install, sign-in, non-interactive modes and test cost:
  `docs/research/other-agent-signals.md`, `docs/research/copilot-vscode-signals.md` ("Free testing"),
  `docs/research/opencode-signals.md` ("What is left"), `docs/research/cursor-gemini-signals.md`.

## Haik's notes (2026-11-06, about 15:30)

- "codex is installed and logged in". Found by the orchestrator: `~/.codex` exists with `.desktop-created` (the
  Codex desktop app), no `sessions` folder yet, and no `codex` on PATH. Find out whether the CLI ships inside the app
  or needs its own install (`@openai/codex` for his user), and whether it reuses the app's login without reading the
  auth file. Never open `auth.json` or anything under `.sandbox-secrets`.
- Test prompts use minimum tokens: tell the agent it is an app test and to answer with one word or `0`, with no
  reasoning (lowest reasoning setting where the agent has one). Find cheaper ways if you can (a local model, a
  one-shot mode, the smallest model).
- VS Code is installed per machine (`C:\Program Files\Microsoft VS Code`), with its `copilot-shim\copilot.exe` on
  PATH; Cursor, OpenCode, Ollama and Antigravity are not installed.

## Part A (now)

1. **Installs for Haik's user, no admin**, each logged with the exact command, version, install folder and how to
   remove it: Copilot CLI, OpenCode 2.x (`@opencode/cli`), Ollama with one small model that can call tools (pick it
   for OpenCode; say why), Cursor (editor; its CLI if it ships separately), VS Code if it is not installed. Antigravity
   only if `docs/tasks/p4-antigravity-research.md` Result says a free test exists by the time you reach it; else list
   it as pending. Before each install, check what is already there. Record anything an installer adds at sign-in
   (Ollama's start-up entry, for example) and leave it as the installer sets it; the orchestrator logs it for Haik.
2. **Logins**: for each agent, whether an existing login can be reused (Copilot CLI and VS Code through Haik's `gh`
   login, for example), by the agent's own documented way. If a login can only be reused by passing a token, pass it
   straight into the child process environment: never print it, log it or write it to a file, and say so in the Log.
   OpenCode with Ollama needs no login. List the rest as **one sign-in sitting for Haik**: each step exact (what to
   open, what to click, which account, which free plan), in the order he does them, with the time each takes. Also
   note each free plan's limits (requests per month), so live tests stay frugal.
3. **Pseudo-console driver** in a new spike, `spikes/agent-drive/` (outside the workspace, its own `Cargo.toml` and
   `Cargo.lock`, the root toolchain): starts one command in a pseudo console (ConPTY), follows a step file (wait for
   text on screen, type text, Enter, Esc, a key, wait), keeps the screen text in a log in the scratchpad, kills only
   its own child tree (a job object), stops at a timeout, and writes a done file with the result, since it is
   started at Medium through `explorer.exe`. Test it on a harmless command (`cmd /c` with a prompt), then on one
   agent's approval prompt and Esc if one runs without a login (OpenCode with Ollama).
4. **Non-interactive modes and VS Code chat**: for each agent, its headless or one-shot mode (`-p`, `exec`, `run`),
   and whether it writes the same session files as the TUI (so a cheap test still proves the source). For VS Code:
   whether `code chat "<prompt>"` (or the current way) starts an agent mode chat from the command line, so no
   computer use is needed. Computer use only as a last resort, and only listed here, not run.
5. **A runbook** in this card's Result: per agent, how a live card starts a working turn, an approval wait, Esc, a
   clean end, a crash (kill by PID), all at Medium, and how it reads the result. Note which steps the driver covers.

## Part B (after `p4-contracts-r2`)

6. `crates/wakeshot-windows/examples/agent-watch.rs`: a `--source` flag for Claude Code, Codex and Copilot CLI (the
   sources that exist), so each later live card adds its own source with one line. Check it with the per-package
   commands only (`cargo clippy -p wakeshot-windows --examples`, `cargo build -p wakeshot-windows --examples
   --locked`), on your own `CARGO_TARGET_DIR`.

## Acceptance checks

- Each installed agent starts at Medium and prints its version; the log has the remove command for each.
- The driver passes its harmless test and one real agent step (or the reason it could not), started through
  `explorer.exe`, with its child tree gone afterwards (checked by PID).
- The sign-in list is complete: every agent is either "works now", "after step N of the sitting", or "not possible
  for free" with the reason.
- Part B: the example builds and runs for each of the three sources on a scratch data folder.

## Out of scope

- Any live test of a source (each `p4-live-*` card does its own).
- Workspace crates other than the example; `Cargo.toml` and `Cargo.lock` of the workspace.
- Sign-ins with Haik's accounts (he does them in the sitting). Buying anything.

## Rules

- Never run git, not even read-only (`git status`, `git diff`, `git --version`). Two workers slipped today.
- Never read Claude's `.key` files, OpenCode's `service.json`, `gh`'s hosts file or any token file. Prompt text of
  real users is never logged; your own test prompts are short and harmless, and the driver logs stay in the
  scratchpad.
- Kill only processes you started, by PID. Start agents and the driver at Medium through `explorer.exe`.
- Never run another keep-awake app; never quit Haik's installed Wakeshot tray.
- Never set `RUSTFLAGS`. Cargo may not be on PATH: `%USERPROFILE%\.cargo\bin\cargo.exe`.
- Facts only from what you ran or read; mark guesses. Plain short English, no em dashes, no emojis. Write Log lines
  as you go.

## Questions for Haik

None. The installs, the model and the driver design were the worker's calls (Result, "Decided by the worker").

## Steps for Haik: the one sign-in sitting (about 10 minutes)

Only Cursor and Antigravity need you. Codex, Copilot CLI, Copilot in VS Code and OpenCode work now. Use your personal
accounts and free plans only; if a page offers a paid plan or a trial, close it or pick Free. Do the steps in this
order, in a normal (not elevated) PowerShell window from the Start menu where a terminal is asked for.

Cursor (about 4 minutes):

1. Start menu > Cursor. On the first screen click "Log In" (or "Sign Up" if you have no Cursor account).
2. A browser opens on cursor.com. Sign in with the account you want for Cursor (GitHub or Google is quickest). If it
   asks for a plan, choose Free (Hobby); skip any Pro trial. Go back to Cursor.
3. If Cursor asks to import VS Code settings or pick a theme, click "Skip" or the default. Check that the bottom left
   shows your account, then close Cursor.
4. In PowerShell run `agent login`. A browser opens; confirm the sign-in with the same account. Back in PowerShell run
   `agent status`: it should name your account. Close the window.

Antigravity (about 5 minutes):

5. In a new PowerShell window run
   `cd C:\Users\haika\AppData\Local\Temp\claude\f7c2c08a8f04f07b\b896b7b1-ede4-4a36-9332-d8c5355d7200\scratchpad\p4-live-harness\work\signin`
   and then `agy` (if not found: `& "$env:LOCALAPPDATA\agy\bin\agy.exe"`).
6. Pick any color scheme and rendering mode. When asked to trust the folder, answer yes (it is the empty test
   folder above).
7. A browser opens. Sign in with your personal Google account (not a work account), accept the terms, and verify
   your age if Google asks.
8. Back in the terminal, check the header shows your email. Type `/usage` and Enter: it should show a Free quota.
   Type `/exit`.
9. Start menu > Antigravity IDE. Open the same folder (File > Open Folder, the path from step 5) and press Ctrl+L
   for the agent panel. It should be signed in already; if it asks, sign in the same way. Close the IDE.

Then tell the orchestrator "sign-in done" and which steps did not go as written.

## Found outside scope

- `p4-contracts-r2`'s "Not verified" point (a folder listing's time for a file held open): one sighting here.
  `Get-ChildItem` showed the driver's open `log.txt` as 0 bytes with its creation time, 6 s after the driver had
  written lines that a read then returned (13:51:25). If the sources' `FindNewest` or `Recent` trust listing sizes
  or times of files an agent keeps open (Codex rollouts, Copilot `events.jsonl`), a write can stay unseen until a
  read or the close; the live cards should time it.
- The Codex CLI TUI writes `~/.codex/config.toml` on its first start (`[tui] screen_reader_detection_done`), and the
  Codex app and CLI share that file and the login. A Codex live card should expect the app's MCP servers
  (`node_repl.exe`) in the CLI's tree, since the app's config turns them on.
- Antigravity's installer put `%LOCALAPPDATA%\agy\bin` first in the user PATH, ahead of `.cargo\bin` and the others
  (harmless today; worth knowing if a name clashes).
- The IDE installer starts the IDE after a silent install (its `[Run]` entry), and the IDE then starts `agy.exe
  --hub`; a script that installs it must expect that and close it.
- VS Code's `copilot` shim comes first on PATH (machine PATH before user PATH), so `copilot` in a terminal runs the
  shim, which then runs the npm CLI. Seen in the driver's tree: shim `copilot.exe` > `cmd.exe` (`copilot.cmd`) >
  `node.exe` (npm loader) > `copilot.exe` (the CLI); the shim stays as a parent, so one session has two
  `copilot.exe` images. The `AnyProcess` image fallback (`p4-live-copilot` Round 2) should match by full path or
  take the lock holder, not the image name alone.

## Log

- 2026-11-06: card written by the orchestrator.
- 2026-11-06: worker started Part A; Status in progress. Part B waits for the orchestrator's go.
- 2026-11-06 13:24: before installs: node 26.13.0, npm 11.20.0 (prefix `%APPDATA%\npm`, folder missing, so no npm
  globals), winget; on PATH only VS Code 1.145.1 (`code`, and its `copilot-shim\copilot.exe`, a launcher that runs a
  `copilot` found on PATH or offers to install the Copilot MSI) and `gh`. No `codex`, `opencode`, `ollama`,
  `cursor`, `agent`, `agy`. Codex desktop app: MSIX `OpenAI.Codex` 26.1103.3517.0 (made `~/.codex` 13:19, names
  only listed), which ships `app\resources\codex.exe` inside the package folder. `CODEX_HOME`, `COPILOT_HOME`,
  `OLLAMA_*`, `XDG_*` unset. Start-up entries, Start menu and per-user uninstall keys saved to
  `<scratchpad>\p4-live-harness\logs\before-state.txt` for a diff after the installs.
- 2026-11-06 13:31: installs started at Medium (a `.cmd` opened with `explorer.exe`): npm `@github/copilot@1.0.106`,
  `@opencode/cli@2.2.5`, `@openai/codex@0.167.0`; winget `Ollama.Ollama` 0.40.15 and `Anysphere.Cursor` (user scope).
  Picks: npm for the three CLIs (each one's documented way, per user, one remove command); the Codex CLI gets its own
  install because the app's `codex.exe` sits in a versioned package folder off PATH. Change it when a live card needs
  the Copilot MSI layout (what VS Code's shim installs) or the app's own `codex.exe`.
- 2026-11-06 13:31 to 13:34: npm installs done at Medium (exit 0 each; npm 11.20 warned that `@opencode/cli`'s
  postinstall is "not yet covered by allowScripts", but its `bin\opencode.exe` is linked to the platform binary, so
  the script ran). Ollama 0.40.15 installed (winget exit 0); its installer started `ollama app.exe` and
  `ollama.exe` (serve) at once. Cursor install running.
- 2026-11-06 13:35 to 13:40: driver `spikes/agent-drive/` written (ConPTY, job object, `vt100` screen, step file,
  serve mode); fmt, clippy and 9 unit tests clean. Harmless tests, first elevated, then at Medium through
  `explorer.exe` (`token.txt`: Medium Mandatory Level): `t1-prompt` (a `cmd` prompt: type, Esc cleared the line,
  Enter, exit), `t2-kill` (`kill PING.EXE` by pid inside the tree, then a step file dropped into `in\`, `end`),
  `t3-leftover` (tree still running at the end: the driver ended it). All `pass`, `active_after` 0; every driver and
  child pid gone afterwards (checked with `Get-Process -Id`). The pseudo console asked for win32-input-mode
  (`CSI ?9001h`) and focus events (`CSI ?1004h`); the driver leaves both off.
- 2026-11-06 13:34: Cursor 3.27.12 installed (winget, user scope, exit 0; not started). Diff of start-up entries:
  Ollama added `Startup\Ollama.lnk` (starts `ollama app.exe` at sign-in; left as the installer set it) and Start menu
  entries; Cursor a Start menu folder; no Run key, task or service. Both added their folder to the user PATH.
  `powercfg /requests`: Ollama holds no power request while idle; it listens on 127.0.0.1:11434 only.
- 2026-11-06 13:41, versions at Medium: Copilot CLI 1.0.106 (npm, and the same through VS Code's shim, which runs
  the npm one), Codex CLI 0.167.0 (npm; the app's own `codex.exe` is 0.167.0-alpha.3.2), OpenCode v2.2.5, Ollama
  0.40.15, Cursor 3.27.12, VS Code 1.145.1. `codex login status`: "Logged in using ChatGPT" (the CLI reads the app's
  login from the shared `~/.codex`; no file opened by me). `gh auth status`: account `HaikAsatryan`, a `gho_` token
  (masked by gh). `code chat --help`: `code chat [options] [prompt]`, `-m agent` is the default mode.
- 2026-11-06 13:43: VS Code's Copilot Chat log (status lines only, grep for sign-in words): "Logged in as
  HaikAsatryan", "copilot token sku: free_limited_copilot". Ollama log: ROCm on the Radeon 890M (gfx1150), 37.7 GiB,
  "vram-based default context" 32768, so no Modelfile is needed for OpenCode's long prompt.
- 2026-11-06 13:44: pulled `qwen3.5:2b` (2.7 GB). One tool call through Ollama's API (`think: false`, a `bash` tool,
  "call the bash tool once with: echo hi"): a correct `bash {"command":"echo hi"}` call in 4.5 s with the model load.
- 2026-11-06 13:45 (orchestrator: Antigravity has a free test, install CLI and IDE if per user): CLI script
  `antigravity.google/cli/install.ps1` read as data first (downloads by a manifest, checks SHA-512, copies to
  `%LOCALAPPDATA%\agy\bin`, runs `agy.exe install` for PATH), then run as a file at Medium: exit 0, `agy` 1.5.2. IDE
  installer read as data first: Inno Setup 6.4, manifest `asInvoker`, header "Antigravity IDE (User)", default folder
  `{userpf}\Antigravity IDE`, so per user. Run at Medium `/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /SP-`: log
  "Administrative install mode: No", root key HKCU, "Installation process succeeded"; it then started the IDE (a
  `[Run]` entry, 13:48:08), which downloaded `~\.gemini\bin\agy.exe` and ran it as `--hub`. I closed the IDE window
  (`CloseMainWindow` on the main pid 7040); all 11 pids of that tree were gone 20 s later. The hub's command line,
  read to find the main pid, holds a CSRF token; it was not copied anywhere and ended with the hub.
- 2026-11-06 13:49: Cursor CLI script `cursor.com/install?win32=true` read as data (per user: `%LOCALAPPDATA%\
  cursor-agent`, adds it to the user PATH, `agent` and `cursor-agent` shims), run as a file at Medium: exit 0,
  `cursor-agent` 2026.11.05-86047df. Antigravity IDE 2.6.0 (VS Code base 1.107.0). Final diff: the IDE added a Start
  menu folder and `...\Antigravity IDE\bin` to the user PATH; agy put `%LOCALAPPDATA%\agy\bin` first in the user
  PATH. No new Run key, start-up item (other than Ollama's), task, service or app package.
- 2026-11-06 13:51 to 13:56, driver on OpenCode 2.2.5 TUI at Medium (`drive\oc1`; test folder with an
  `opencode.jsonc`: model `ollama/qwen3.5:2b`, one rule `* * ask`, share and update off): the TUI started in 4 s and
  showed "Build · qwen3.5:2b Ollama" (the Ollama plugin found the local model with no login). Tree: the TUI
  `opencode.exe` and its background service `opencode.exe`, plus a short `node.exe`, `cmd.exe` and `conhost.exe` at
  start. A prompt was typed and sent; the model then thought for 3 min (6000 tokens at 27 tok/s, none shown) with no
  tool call, so `qwen3.5:2b` is too slow for OpenCode (it thinks by default, and OpenCode sends no `think: false`).
  Esc: one Esc did nothing in 20 s; two Esc 0.3 s apart interrupted ("interrupted" under the prompt; Ollama logged
  "aborting completion request due to client closing the connection"). `end`: the driver ended both `opencode.exe`;
  all 6 pids gone (checked by pid). Fixed in the driver after this run: screen text is one line per row (OpenCode
  paints full rows, which `vt100`'s `contents()` joined into one line); test added.
- 2026-11-06 13:57: swapped the model for `qwen3:4b-instruct` (2.5 GB, Qwen's non-thinking 2507 instruct; API tool
  call right in 11 s with the load, no thinking field) and removed `qwen3.5:2b` (`ollama rm`).
- 2026-11-06 13:57 to 14:04, driver run `drive\oc2` at Medium, the real agent step: TUI up in 7 s; prompt typed;
  "Permission required ... `$ echo hi` ... Allow once / Always allow / Reject" on screen 24 s after Enter (seen by
  `wait`). Left 10 s, then Esc twice: "The user declined this tool call", turn "interrupted". A second prompt got a
  plain answer with no tool call (the model skipped it), so that step failed by timeout as it should and serving went
  on. A third prompt: approval shown, Enter (Allow once is the default) ran `echo second` (OpenCode ran it through
  `powershell.exe`, seen in the tree), output `second`, answer `0`, turn done in 30 s. `/exit` + Enter: the TUI
  ended with exit 0 in 0.1 s; its background service `opencode.exe` stayed (by design), the driver ended it at `end`.
  `done.json` `pass`, `active_after` 0; all 8 pids (2 `opencode.exe`, `powershell.exe`, `cmd.exe`, `node.exe`, 2
  `conhost.exe`, the driver) gone, checked by pid.
- 2026-11-06 14:05, one-shot modes at Medium in `work\oneshot`, prompt "This is an app test. Do not use tools. Answer
  with 0 only.": `opencode run --model ollama/qwen3:4b-instruct`: `0`, 16.5 s, wrote `opencode.db` (+wal) and its
  log, the same database the TUI uses; it left its background service running, stopped with `opencode service stop`
  (exit 0, pid gone). `codex exec --skip-git-repo-check -m gpt-6-luna -c model_reasoning_effort="low"`: `0`, 5 s,
  "tokens used 3,724", wrote `sessions\2026\11\06\rollout-...-<thread id>.jsonl` (the TUI's path) and made
  `thread-writer-locks\`. So the ChatGPT login of the app works for the CLI. `copilot -p ... -s --trust-folder`
  (no token passed: the CLI took `gh`'s login itself, its documented last fallback): `0`, 11 s, wrote
  `session-state\<id>\events.jsonl`, `workspace.yaml`, `checkpoints\`, `rewind-file-snapshots\` and a
  `.session-operation-locks\<id>.lock`. Names and sizes only were listed, no content read.
- 2026-11-06 14:07, TUI start under the driver, no prompt, then `kill-tree` as a crash: Codex 0.167.0 drew its home
  screen ("Ask Codex to do anything", GPT-6.1-Sol default, no trust question in a non-git folder; tree `cmd`,
  `node`, 3 `codex.exe`, 2 `node_repl.exe` from the app's MCP config, `node`, 2 `conhost`). Copilot 1.0.106 (through
  VS Code's shim) drew its banner and "Confirm folder trust" (1 Yes, 2 Yes and remember, 3 No); its tree ran `gh.exe`
  (the login fallback), `reg.exe` and `powershell.exe`. Both `pass`, every pid gone. Side effects: the Codex TUI
  added `[tui] screen_reader_detection_done` to `~/.codex/config.toml` (43 bytes, 14:07:00); nothing was trusted.
  Test sessions left in Haik's homes: one Codex rollout, two Copilot `session-state` folders, OpenCode's test
  sessions in `opencode.db` (all from today's installs; left so the apps' own indexes stay consistent).
- 2026-11-06 14:08: driver gained `wait-new <secs> <text>` (waits for one more copy of an end marker, since old turns
  keep theirs on screen: after the third OpenCode turn a `wait-gone` of the busy footer passed in 0.2 s, too early to
  trust). Test added; fmt, clippy, 11 tests clean.
- 2026-11-06 14:10: `kill` takes a pid too (checked in the tree before the kill). Rerun at Medium: `t2-kill` (by
  image) and `t4-killpid` (by pid, from a step file dropped in while serving): both pass, every pid gone.
- 2026-11-06 14:12: `agent --help` (Cursor CLI: `login`, `status`, `-p`, `--model`, `--list-models`) and `agy --help`
  (`-p`, `--effort`, `--model`) at Medium; nothing left running. Deleted the IDE installer download (200 MB).
- 2026-11-06 14:13: Part A done; Status blocked on the sign-in sitting and the go for Part B.

## Result

Part A done (2026-11-06 14:13). Waiting: Haik's sign-in sitting (above, about 10 minutes) for Cursor and
Antigravity, and the orchestrator's go for Part B.

### Installed for Haik's user (no admin; each run at Medium, versions printed at Medium)

| Agent | Version | How | Folder | Remove |
|---|---|---|---|---|
| Copilot CLI | 1.0.106 | `npm install -g @github/copilot@1.0.106` | `%APPDATA%\npm\node_modules\@github\copilot` | `npm uninstall -g @github/copilot` |
| OpenCode | 2.2.5 | `npm install -g @opencode/cli@2.2.5` | `%APPDATA%\npm\node_modules\@opencode\cli`; data `~\.local\share\opencode` | `npm uninstall -g @opencode/cli` (data stays; `opencode uninstall` removes more) |
| Codex CLI | 0.167.0 | `npm install -g @openai/codex@0.167.0` | `%APPDATA%\npm\node_modules\@openai\codex`; shares `~\.codex` with the app | `npm uninstall -g @openai/codex` |
| Ollama | 0.40.15 | `winget install --id Ollama.Ollama --exact --source winget --version 0.40.15` | `%LOCALAPPDATA%\Programs\Ollama`; models `~\.ollama` | `winget uninstall --id Ollama.Ollama` (or `unins000.exe /SILENT`); delete `~\.ollama` for the model |
| Model `qwen3:4b-instruct` | Q4_K_M, 2.5 GB | `ollama pull qwen3:4b-instruct` | `~\.ollama\models` | `ollama rm qwen3:4b-instruct` |
| Cursor editor | 3.27.12 (winget's newest; it updates itself) | `winget install --id Anysphere.Cursor --exact --source winget --scope user` | `%LOCALAPPDATA%\Programs\cursor` | `winget uninstall --id Anysphere.Cursor` |
| Cursor CLI | 2026.11.05-86047df | `cursor.com/install?win32=true`, read, then run as a file | `%LOCALAPPDATA%\cursor-agent` (`agent`, `cursor-agent`) | delete `%LOCALAPPDATA%\cursor-agent` and its user PATH entry |
| Antigravity CLI | 1.5.2 | `antigravity.google/cli/install.ps1`, read, then run as a file | `%LOCALAPPDATA%\agy\bin` | delete `%LOCALAPPDATA%\agy` and its user PATH entry |
| Antigravity IDE | 2.6.0 (user installer) | `Antigravity IDE.exe` 2.6.0 from `antigravity.google/download`, `/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /SP-` | `%LOCALAPPDATA%\Programs\Antigravity IDE`; it fetched `~\.gemini\bin\agy.exe` | `"%LOCALAPPDATA%\Programs\Antigravity IDE\unins000.exe" /SILENT` |

Already there: VS Code 1.145.1 (per machine), the Codex app (MSIX 26.1103.3517.0), `gh`, node 26.13, npm 11.20.
Added at sign-in by an installer: only Ollama's `Startup\Ollama.lnk` (starts `ollama app.exe`, which serves on
127.0.0.1:11434, no power request while idle). Also added: user PATH entries for npm's folder (already listed),
Ollama, Cursor, Cursor CLI, the Antigravity IDE and `agy` (first); Start menu entries for Ollama, Cursor, the IDE.
No Run key, task, service or app package. Model pick: `qwen3:4b-instruct`, Qwen's non-thinking 4B: right tool calls,
on the 890M GPU an approval prompt in 24 s and a full OpenCode turn in 30 s; `qwen3.5:2b` (tried first, removed)
thinks for minutes. Change it when a newer small non-thinking tool model is in Ollama's library, or OpenCode gains a
setting that turns thinking off.

### Logins

| Agent | State | How |
|---|---|---|
| Codex CLI | works now | the app's ChatGPT login in the shared `~\.codex` (`codex login status`; a `codex exec` answered) |
| Copilot CLI | works now | `gh`'s login (`HaikAsatryan`, Copilot Free), the CLI's documented last fallback; no token passed by me; a `copilot -p` answered |
| Copilot in VS Code | works now | VS Code is signed in (`free_limited_copilot`, from its log) |
| OpenCode + Ollama | works now | no login (local model) |
| Cursor editor and CLI | after steps 1 to 4 of the sitting | Cursor account, Free plan |
| Antigravity CLI and IDE | after steps 5 to 9 | personal Google account, Free plan |
| Gemini CLI | not possible for free | out of v1 (Haik); enterprise or paid key only since 2026-06-18 |

Free plan limits, to keep live tests frugal (read 2026-11-06): Copilot Free: auto model only, 2000 completions a
month, chat and agent use from "an allowance of GitHub AI Credits" with no number (GitHub docs); VS Code and the CLI
share it. Codex: Haik's ChatGPT plan, limits not checked (`/status` in the TUI shows them); one `exec` with
`gpt-6-luna` at low effort used 3,724 tokens. Cursor Free: "Limited agent requests", no number (cursor.com/pricing).
Antigravity Free: a weekly quota of unpublished size (`docs/research/antigravity-signals.md`). OpenCode with Ollama:
none.

### The driver: `spikes/agent-drive/`

`agent-drive <folder>` runs `<folder>\steps.txt`: settings, one `run <command line>`, then steps. Settings: `size
<cols> <rows>` (120 40), `timeout <s>` (600), `cwd <folder>`, `env <name> <value>`, `unset <name>`, `env-from <name>
<program> [args]` (value from the program's stdout, only into the child's environment, never logged), `type-delay
<ms>` (15), `raw on`. Steps: `wait <s> <text> [<text>...]`, `wait-gone <s> <text>`, `wait-new <s> <text>` (one more
copy than at the step's start: a turn's end marker), `quiet <s> <ms>`, `type <text>`, `paste <text>`, `send
"<escaped>"`, `key <name> [n]` (enter, kenter, esc, kesc, tab, shift-tab, arrows, home, end, pgup, pgdn, delete,
backspace, space, ctrl-a..z), `enter`, `esc`, `sleep <s>`, `snap [label]`, `kill <pid or image.exe>` (one process,
checked to be in the tree), `kill-tree`, `wait-exit <s>`, `pids`, `serve` (then run each `<folder>\in\*.txt` dropped
in, results in `in\<name>.result`), `end`. Output in the folder: `started.json` (driver and child pids), `screen.txt`
(one line per row, every 250 ms), `log.txt` (steps, tree changes, terminal queries, screen snaps), `done.json`
(`result` pass, fail, timeout or error; every pid seen with its image; `active_after`). It drops `CLAUDECODE`,
`AI_AGENT` and `CLAUDE*` (except `CLAUDE_CONFIG_DIR`) from the child's environment, answers the cursor, device,
size and color queries that reach it, and at the end always ends the child's job (kill on close), then waits until
the job is empty. Start it at Medium with `<scratchpad>\p4-live-harness\run\start-drive.ps1 -Folder <folder>` (writes
a `.cmd`, opens it with `explorer.exe`; `token.txt` shows the level). Built exe: `<scratchpad>\p4-live-harness\bin\
agent-drive.exe` (build: `cargo build --release --locked --target x86_64-pc-windows-msvc` in the spike, own
`CARGO_TARGET_DIR`).

Decided by the worker: own `vt100` screen over cutting escapes (TUIs redraw single cells, so words split); a job
object with the child made suspended (no grandchild escapes); null standard handles as Windows Terminal passes;
kitty keyboard left unanswered so keys stay plain; files in and out (the driver runs at Medium, the caller cannot
wait on it). Change it when a TUI needs win32-input-mode keys (send them with `send`), or a live card needs a
person's exact key timing.

### Runbook for the live cards (all at Medium through the driver unless named)

| Agent | Start | Working turn | Approval wait | Esc | Clean end | Crash | Read the result |
|---|---|---|---|---|---|---|---|
| OpenCode 2.x | `run <npm>\node_modules\@opencode\cli\bin\opencode.exe` in a folder with `opencode.jsonc` (model `ollama/qwen3:4b-instruct`, `permissions` `* * ask`); `wait 60 "Ask anything"` | `type <prompt>`, `enter` | `wait 300 "Permission required"`; approve with `enter` (Allow once) | two `esc` 0.3 s apart (one does nothing); at a prompt it declines | `type /exit`, `enter`, `wait-exit 20`; then the service: `opencode service stop` or the driver's `end` | `kill <TUI pid>` (the service lives on); `kill-tree` for both | screen, `opencode.db`; one-shot `opencode run --model ollama/qwen3:4b-instruct "<p>"` writes the same db |
| Codex CLI | `run cmd /d /c codex`; `wait 30 "Ask Codex"` | `type`, `enter` (type-delay 15 ms or more: Codex reads fast keys as a paste) | a command outside the sandbox (not run here) | `esc` (not run here) | `/exit` (not run here) | `kill <pid>` of the TUI `codex.exe` (three run; pick by `pids` and parent) | rollout in `~\.codex\sessions\...`; one-shot `codex exec -m gpt-6-luna -c model_reasoning_effort="low" "<p>"` writes the same |
| Copilot CLI | `run cmd /d /c copilot`; `wait 30 "Do you trust"`, `enter` (1, this session only) | `type`, `enter` | a shell tool prompt (not run here) | `esc` (not run here) | `/exit` (not run here) | `kill <pid>` of the inner `copilot.exe` (the shim is one too) | `~\.copilot\session-state\<id>\events.jsonl`; one-shot `copilot -p "<p>" -s --trust-folder` writes the same |
| Claude Code | as `p4-live-claude` (`ccdrive.py`), or `run cmd /d /c claude` here | `type`, `key kenter` (ccdrive's finding) | its dialogs: digits or arrows, `enter` | `esc` | `/exit` | `kill claude.exe` by pid | status file, transcript |
| Cursor CLI | after the sitting: `run cmd /d /c agent` (not run) | `type`, `enter` | not run | not run | not run | `kill <pid>` of its `node.exe` | hooks only (`p4-hooks-design`); one-shot `agent -p "<p>"` (`--model`, `--list-models` to pick the cheapest) |
| Antigravity CLI | after the sitting: `run <LOCALAPPDATA>\agy\bin\agy.exe` (not run) | `type`, `enter` | not run | not run | `/exit` | `kill agy.exe` by pid | hooks only; one-shot `agy -p "<p>" --effort low` |
| VS Code chat | not the driver: `code -n <trusted scratch folder>`, then `code chat -r -m agent "<p>"` (submits the prompt; read in code, not run) | that command | needs a click: computer use, last resort, not run | computer use only | close the window it opened, by its pid | `kill` the `Code.exe` pid it started (never Haik's window) | `chatSessions` files (`copilot-vscode-signals.md`) |
| Cursor editor, Antigravity IDE | not the driver: `cursor <folder>`, `antigravity-ide <folder>` | the agent panel needs clicks: computer use only (listed, not run) | computer use | computer use | close by its main pid | kill its main pid | Cursor `main.log` lines or hooks; Antigravity hooks |

Every card: test prompts say "This is an app test" and ask for one word or `0`, with the lowest effort the agent has;
the driver's `done.json` lists every pid to check gone with `Get-Process -Id` afterwards.

### Checks

- Each installed agent started at Medium and printed its version (log `versions.log`, `cursor-cli-install.log`):
  pass. Remove commands: the table above.
- Driver: harmless tests at Medium (`t1-prompt`, `t2-kill` by image, `t3-leftover`, `t4-killpid` by pid): all pass,
  `active_after` 0, every pid gone by `Get-Process -Id`. Real agent step: OpenCode's approval prompt seen, Esc
  declined it, a later approval taken with Enter, `/exit`; Codex and Copilot TUIs drawn under the driver and killed
  as a crash. All trees gone (checked by pid). Pass.
- Sign-in list complete: every agent is "works now", "after steps N of the sitting" or "not possible for free".
- Spike: `cargo fmt --check`, `cargo clippy --locked --all-targets --target x86_64-pc-windows-msvc -- -D warnings`,
  `cargo test --locked` (11 tests): clean. Nothing built inside the workspace; no workspace file touched.
- Part B: not started.

### Files

- New: `spikes/agent-drive/` (`Cargo.toml`, `Cargo.lock`, `src/main.rs`, `src/conpty.rs`, `src/script.rs`,
  `src/term.rs`). Changed: this card, `docs/research.md` (new section "Live harness: agents installed, logins, a
  pseudo console driver").
- Scratchpad `p4-live-harness\`: `bin\agent-drive.exe`, `run\start-drive.ps1`, `run\start-medium.ps1` and the
  scripts, `drive\*` logs, `logs\*`, `work\oc-drive` (the OpenCode test folder), `work\signin` (for the sitting),
  `target\` (kept for Part B, deleted at its end).

### Left in Haik's agent homes (test data from today's installs)

One Codex rollout (`~\.codex\sessions\2026\11\06`, the `exec`), two Copilot `session-state` folders, OpenCode test
sessions in `opencode.db`, and `[tui] screen_reader_detection_done` in `~\.codex\config.toml`. Left so the apps'
own indexes stay consistent; they show as test sessions in each app.
