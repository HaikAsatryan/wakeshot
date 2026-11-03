# Lid and sleep spike

Phase 0 throwaway code for card `docs/tasks/p0-spike-lid-sleep.md`. It answers: what keeps a closed laptop running
and what puts it to sleep when work ends, whether wake timers wake this laptop, when lid notifications come, whether
lock and a sound work at lid close, how to count external screens, how to read a temperature, and whether a held
request survives a manual sleep and resume. Findings live in `docs/research.md`, section "Lid and sleep spike".

Not product code.

## Build

```
%USERPROFILE%\.cargo\bin\cargo.exe build --release
%USERPROFILE%\.cargo\bin\cargo.exe test
```

## Run

`target\release\lid-sleep-spike.exe <mode>`. Every run writes a log to `out\<mode>-<time>.log` (each line flushed,
so a sleep or kill loses nothing). Runs that test something print `RESULT:` lines near the end and finish with
`DONE`.

| Mode | What it does | Writes |
| --- | --- | --- |
| `read` | Power capabilities, hibernate, `powercfg /a`, lid, sleep, display off, hibernate after, wake timer setting, sign-in and energy saver values, first lid and display callbacks, active screens (display config API and WMI), thermal zones (PDH and WMI), `CallNtPowerInformation` reads, SeShutdownPrivilege, lock state, Lenovo services. | nothing |
| `heat [--seconds N] [--load-threads T] [--load-at S] [--load-seconds L] [--heat-limit C]` | Thermal zone every 2 s, with an optional CPU load. The load stops early at C (default 85). | nothing |
| `timer-check [--after S]` | Arms a wake timer (absolute due time, resume TRUE) while awake; logs what `SetWaitableTimer` says and `powercfg /waketimers` (needs admin). | nothing |
| `audio-read` | Default playback device per role, every active playback endpoint (form factor, volume, mute), the audio sessions on each (the System Sounds session included). | nothing |
| `sound-test [--chime-only]` | Lid open: the Phase 0 alias sound, then our chime on the picked device, with loopback listeners on every active playback endpoint. Plays two short sounds. | nothing |
| `lid [--end release\|screen-off\|sleep\|hibernate\|none] [--work S] [--no-lock] [--no-sound] [--sound chime\|alias] [--alias-after S] [--chime-after S] [--listen] [--heat-limit C]` | Plan copy with lid action Do nothing, holds System and Execution requests, ticks every second. At lid close it plays a sound (`chime`, the default: our own sound on the picked device, started before the lock; `alias`: the Phase 0 `PlaySound` call) and locks. `--alias-after` and `--chime-after` play the other sound S seconds after the close. `--listen` logs what reaches each playback device (WASAPI loopback). `--work` seconds later (default 60), if the lid is still closed: releases the requests, puts the user's plan back, then runs the end method. `none` holds until the lid opens (docked check). Work also ends early, in every `--end` mode, when the hottest thermal zone reaches `--heat-limit` C (default 85; read every 5 s). Ends 20 s after the lid opens, or after `--max-minutes` (default 12). | a copy only |
| `suspend --method sleep\|hibernate\|screen-off [--wake-after S]` | Lid open, no request. Calls `SetSuspendState` or broadcasts `SC_MONITORPOWER` after a 10 s countdown. Ends 20 s after you touch the PC. | nothing |
| `wake [--after S] [--wake-timers 0\|1\|2]` | Arms a wake timer (default 120 s); you sleep the PC with Start. `--wake-timers` sets "Allow wake timers" on a plan copy for the test. | a copy only, if `--wake-timers` |
| `resume [--retake] [--request power\|stes] [--idle S]` | Plan copy with sleep after and display off set to S (default 60 s). Holds a request; you sleep and wake the PC; then 4 minutes of hold (does the PC stay up?) and 3 minutes without (control: it should sleep). | a copy only |
| `restore` | Undoes what `out\journal.txt` lists: switches away from a spike copy and deletes it. | as needed |
| `limited <mode> ...` | Starts the mode with Explorer as parent (unelevated Medium token) and waits. For tests from an elevated session. | as the mode |

`lid`, `suspend`, `wake` and `resume` refuse to run elevated unless `--allow-elevated`. For Haik's steps, run them
from `spikes\morning-shell.cmd` opened in File Explorer (normal Medium token; see `docs/phases/00-morning-steps.md`).
`sound-close-1.cmd` and `sound-close-2.cmd` start the two lid closes of card `docs/tasks/p1-lid-sound.md`; open
them from File Explorer. Device rule for the chime: the default device, unless it is a screen's audio (HDMI or
DisplayPort form factor); then the speakers. Dry run options for tests without a lid or a sleep: `--simulate-close-after S` (lid), `--simulate-resume-after S`
(resume), `--dry` (suspend).

A plan copy is refused while `spikes\power-settings\out\journal.txt` exists or while the active plan is a spike copy
(name starts with "Wakeshot spike"): a copy of the other spike's copy would carry its test values, and restore would
put that copy back instead of the user's plan.

## Safety

- The user's own plan is never written. Tests that change settings make a copy, write `out\journal.txt` right after
  (the copy's GUID does not exist before; a kill in that short gap leaves an unlisted, inactive copy named like the
  user's plan), and switch back and delete it at the end and on Ctrl+C (tested). A closed classic console window does
  the same through the console handler (tested by the safety review); a closed Windows Terminal tab is not tested.
  Sign-out and shutdown do not reach the handler (the exe loads user32). If a run is killed, the copy stays active
  (with lid Do nothing in `lid` mode) until `restore` (tested).
- `restore` never switches to a plan that is gone or is a spike copy; it falls back to Balanced, then to any other
  plan, before it deletes the active copy.
- `lid` re-checks the lid right before the end method and skips it if the lid is open.

## Notes

- `limited` gives a token that cannot be opened with `TOKEN_ADJUST_PRIVILEGES`, so `SetSuspendState` cannot be
  tested through it. A process started by Explorer itself (for example `morning-shell.cmd` opened from File
  Explorer) can enable SeShutdownPrivilege. On this laptop Windows Terminal opens elevated, so it is not "normal".
- Wake timers use an absolute UTC due time: since Windows 8 a relative one does not count down while the PC sleeps.
- A gap is a tick that came at least 2 s late. `Slept` means unbiased interrupt time skipped too; `Paused` means it
  ran on while our process did not.
