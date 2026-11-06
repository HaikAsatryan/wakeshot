# Live harness: agents run live tests without Haik

Phase: 4. Status: in progress (Part A). Needs Haik: yes, once, after Part A: one short sign-in sitting with the
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

## Found outside scope

## Log

- 2026-11-06: card written by the orchestrator.
- 2026-11-06: worker started Part A; Status in progress. Part B waits for the orchestrator's go.

## Result
