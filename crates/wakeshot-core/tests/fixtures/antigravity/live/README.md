# Antigravity hook inputs, live

Real inputs of one CLI turn, as `agy` 1.5.2 handed them to a hook on stdin (2026-11-06, Antigravity CLI 1.5.2, Gemini
3.8 Flash on the Free plan). Kept by a scratch capture hook beside Wakeshot's, then redacted: the chat id is a made-up
UUID, every path is under `C:/Users/dev`, and `lastUserInput` (the prompt) is replaced. Every key, its order (`agy`
sorts them), its type and the compact form with no line end are as captured.

The turn: a prompt asked for `ping -n 20 127.0.0.1`, approved after a 60 s wait.

1. `turn-1-pre-invocation.json`: the first model call (`invocationNum` 0).
2. `turn-2-pre-invocation.json`: the model call after the command started (`invocationNum` 1).
3. `turn-3-stop-background.json`: `agy` moved the command to the background after about 10 s and the loop ended:
   `fullyIdle` false, `terminationReason` `NO_TOOL_CALL`.
4. `turn-4-pre-invocation-woken.json`: the command ended and woke the agent by itself, a new loop (`invocationNum` 0).
5. `turn-5-stop-idle.json`: that loop's end, `fullyIdle` true.
