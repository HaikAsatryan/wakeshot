# Antigravity hook inputs

What Antigravity hands Wakeshot's hook on stdin, one file per event. Built by `p4-antigravity-source` (2026-11-06)
from the field names in `agy` 1.5.2's own descriptors (`exa.hooks_pb.HookArgsCommon`, `PreInvocationHookArgs`,
`StopHookArgs`; `docs/research.md`, "Antigravity hook payload form"), with made-up values: no hook had run yet.
`p4-live-antigravity` replaces them with real captures, prompt and model output replaced.

- `cli-pre-invocation.json`, `cli-stop-idle.json`, `cli-stop-background.json`: one CLI chat. The idle stop holds
  `finalModelOutput`, which may be deleted as an internal field before the hook gets it (not verified); the
  background stop has `fullyIdle` false, which `agy` writes (protojson `EmitUnpopulated`).
- `ide-pre-invocation.json`, `ide-stop-canceled.json`: one IDE chat, pretty printed, its id in upper case where the
  hook must lowercase it, and Esc's reason.

Real captures are in `live/` (`p4-live-antigravity`). They differ from the made-up files above in ways the hook
ignores: a real `Stop` has no `finalModelOutput`, its `terminationReason` was `NO_TOOL_CALL`, the paths use `/`, and
the form is compact with sorted keys. Esc in the CLI runs no `Stop` at all, so `ide-stop-canceled.json` stays a
made-up case. The made-up files stay for what the real ones do not show (pretty printing, an upper-case id).

`log/cli-20261106_185448.log` (`p4-antigravity-exact-end`): a log in the shape of `agy` 1.5.2's own
(`%USERPROFILE%\.gemini\antigravity-cli\log\`). Lines 1 to 4 and the cancel and "Interrupt cleared" lines of chat
`1ad3ac30-...` at 18:56 and 18:59 are copies of a test TUI's real lines (no prompt, path or account in them); the
second chat's line, the "Cancelling steps" line, the warning's text and every prompt line are made up. The prompt lines hold the test marker
instead of a prompt; one prompt has a second line and one quotes a cancel line, as a real prompt could.
