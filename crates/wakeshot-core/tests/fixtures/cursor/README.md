# Cursor hook inputs

What Cursor hands Wakeshot's hook on stdin, one file per event. Built by `p4-cursor-source` (2026-11-06) from the
field names in Cursor's own code (editor 3.27.12: `triggerStopHook` and `executeHookForStep` in
`workbench.desktop.main.js`; CLI 2026.11.05-86047df), with made-up values: no hook had run yet. `p4-live-cursor`
replaces them with real captures, prompt and email replaced.

- `editor-before-submit-prompt.json`, `editor-stop-completed.json`, `editor-stop-aborted.json`,
  `editor-session-end.json`: one editor chat; its ids in upper case where the hook must lowercase them.
- `cli-before-submit-prompt.json`: the CLI's form, pretty printed, with its date-shaped version.
