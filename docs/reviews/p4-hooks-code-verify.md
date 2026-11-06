# Verify: agent hooks code review, M1

Check of the M1 fix of `reviews/p4-hooks-code.md`, done by `p4-live-cursor` (its Result, "Review fixes"). Read:
`crates/wakeshot-cli/src/hook.rs` (`run`, `hook`, `input_on_command_line`, `apply`, `end_chats`, the fake and the new
test), `crates/wakeshot-core/src/sources/hooked.rs` (`walk`, `Walk`), `crates/wakeshot-windows/src/process.rs`
(`command_line_contains`), `crates/wakeshot-windows/src/agent_hooks.rs` (`remove_command`). Nothing run; no git; no
real `.cursor` or `.gemini` folder touched.

## M1: the command-line privacy check ran only when the walk found an agent

**Fixed.**

- `hook.rs:325-328`: the check now runs on `walk.parent` right after the walk and before the `walk.agent` gate.
  `hooked::walk` fills `parent` with the hook's first listed parent whatever its image (`hooked.rs:632`), so a host
  process the walk does not know is still checked.
- `hook.rs:329-341`: a hit with no agent process logs one line, writes no file, and removes the plugin: at Medium
  through `agent_hooks::remove_command` (which checks the level again, `agent_hooks.rs:763-788`) plus `end_chats` for
  the agent's chat files; above Medium through the Medium copy's `hook <agent> remove`, which needs no checked values.
  That is the reviewer's suggested fix.
- `hook.rs:301-314`: a failed process list logs one line and returns with nothing written. The reviewer allowed "or
  skip only then". The leak in that run has already happened by the time the hook runs, the list needs no rights, so a
  lasting failure is unlikely, and the next run checks again. Acceptable.
- Test `the_command_line_check_runs_without_an_agent_process_too` (`hook.rs:1233-1271`) covers both paths: Medium
  removes the plugin and the other chat's file and keeps `notes.json`; elevated touches nothing and hands off exactly
  `hook cursor remove`. The card says it failed before the fix. The fake asserts the check is asked about the hook's
  own parent with the agent's payload key.

## New blockers or majors from the fixes

None found. Read for this: the M1 path above, `end_chats` (Medium only, plain files named by the id rule, links
skipped), `remove_command` (level checked, fixed names), and the 8.3 short path for Antigravity (the long parts of
the path, `HaikAsatryan.Wakeshot` and a user folder with a space, are made once, not at each update, so their short
names stay; changing a short name needs admin).

Small notes, not findings:

- The list-failure path has no test. It is a log line and a return; fine to leave.
- If the hook's own entry is missing from the list, `walk` returns no parent and the check is skipped without its
  own log line (only "no Cursor process above the hook"). It should not happen, since the hook lists itself.
- The check looks for the payload key as plain text. A Cursor that moved to `-EncodedCommand` (base64) would pass it
  while audit logs still keep the payload. The CLI `2026.11.05` uses no `-EncodedCommand` (grep of its JS, earlier
  review). Change it when a Cursor version is seen using it: decode and search, or count the flag as a hit.
