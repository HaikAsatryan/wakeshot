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
