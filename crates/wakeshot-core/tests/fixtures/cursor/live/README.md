# Cursor hook inputs, live

What the Cursor editor 3.27.12 and the Cursor CLI 2026.11.05-86047df handed a hook on the test laptop, 2026-11-06. The
capture hook saved each input already redacted, so no prompt, e-mail or path reached the disk. Kept as captured: the
keys and their order, ids' shape, `model`, `composer_mode`, `status`, `loop_count`, the token counts, `reason`,
`duration_ms`, `is_background_agent`, `final_status`, `cursor_version`, which values were `null`, and each array's
length. Placeholders: every id is fake (each chat keeps one id across its files), and `prompt`, `user_email`,
`workspace_roots`, `transcript_path`, `model_id` and `model_params` hold made-up values of the captured type. Written
compact, as `JSON.stringify` writes: not verified, only each input's length was kept.

- `r2-*`: one editor chat. A prompt, its `stop` `completed`; a second prompt (count to 10000), cancelled with the Stop
  button: `stop` `aborted` with that prompt's `generation_id`.
- `r3-*`: the same window, three chats. `r3-1`: an empty chat closed (`sessionEnd`, `final_status` `none`, empty
  `generation_id`). Chat A's prompt, then Ctrl+N and chat B's prompt; B's tab closed with Ctrl+F4 while B ran
  (`r3-4`: `sessionEnd`, `user_close`, `final_status` `generating`, B's turn id); then A's `stop` and B's `stop`, both
  `completed`, when each turn ended in the background.
- `cli-session-end.json`: the CLI in `-p` mode. With only Wakeshot's plugin, the CLI fired `sessionEnd` and no
  `beforeSubmitPrompt` or `stop`, in `-p` and in interactive mode.
- `state-working.json`: the chat file Wakeshot's hook wrote for a prompt (pid and start fake).
