# Claude Code transcripts made by hand

Transcripts for cases the live sessions in `../live/` do not cover, for `tests/claude_transcript.rs` (card
`docs/tasks/p4-claude-transcript.md`). Made 2026-11-03 by a script in the card's scratch folder from the redacted
live lines: every line is a live line with the same keys in the same order and the same redaction (`../live/README.md`,
"Redaction"); only the values named below were changed. The fake UUIDs keep the numbers of the live case each line
came from, so one file can mix them.

| File | Built from | What changed | The test expects |
|---|---|---|---|
| `broken-line.jsonl` | `03-loop-1m` lines 1 to 17 | line 10 (the first fire) cut short before its `cron` key, as a crash would leave it | the problem `transcript`/`line`, named only while the session holds anyway; the job still moved on by the fire at line 15 |
| `unknown-kind.jsonl` | `03-loop-1m` lines 1 to 9, 10, 19, 20 | the last three get kinds not seen yet: `subtype` `scheduled_task_fire_preview`, `type` `assistant_summary` and `user_echo` | no problem; the job keeps its `nextFireAt` |
| `missing-field.jsonl` | `05-reminder` lines 10, 11, 19 | no `id` in the `CronCreate` result, no `taskId` in the fire | no job; the first problem, `transcript`/`CronCreate result` |
| `seven-days.jsonl` | `03-loop-1m` lines 5, 6, then line 10 three times | the fires are at 2026-11-10 14:18:22.300, 14:19:22.300 and 14:20:22.300 UTC; the job was made 2026-11-03 14:19:24.194 | the first two move the job on, the third (7 days after its making) ends it; a process resumed at 14:19:30 that day drops it |
| `deleted.jsonl` | `03-loop-1m` lines 1 to 7, 19, 20 | a `CronDelete` of the job whose result is an error, one of an id no job has, then the real one | the job stays until the real delete |
| `carry-after-clear.jsonl` | `06-clear-during-loop.2` lines 1 to 9 | the carry line also has a one-shot in `tasks`, a daily job in `otherArmed`, a `wakeup` and an `ended` id, as Claude Code 2.1.317 can write it | read alone (Wakeshot started after the `/clear`): the four due times |
| `resume.jsonl` | `05-reminder` 10, 11; `08-sdk-bg-shell` 3 to 5; `09-resume-clean` 28, 29, 34, 35; `05-reminder` 10, 11 again; `13-sdk-resume-kill` 18; `09-resume-clean` 50 | one SDK session id, `entrypoint` `sdk-cli` and one `cwd` on every line; the second one-shot is `34 18 3 11 *` at 14:23:41 UTC; the stopped notice names the shell of line 5 and comes at 14:25:01.200; the fire at 14:29:46.100 | for a process started at 14:25:00.000 UTC: the missed one-shot, the wakeup and the shell are gone, the `*/5` job counts from the start |
