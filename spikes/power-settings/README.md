# Power settings spike

Phase 0 throwaway code for card `docs/tasks/p0-spike-power-settings.md`. It answers: can a normal user write the lid
action and the sleep timer, how fast can we see a change someone else makes, what a temporary plan copy does, what
we can read without admin, and whether Windows ends a power request on battery. Findings live in
`docs/research.md`, section "Power settings spike".

Not product code.

## Build

```
%USERPROFILE%\.cargo\bin\cargo.exe build --release
%USERPROFILE%\.cargo\bin\cargo.exe test
```

## Run

`target\release\power-settings-spike.exe <mode>`. Every run writes a log to `out\<mode>-<time>.log` (each line
flushed, so a sleep or kill loses nothing).

| Mode | What it does | Writes |
| --- | --- | --- |
| `read` | Active plan, every plan, lid, sleep, energy saver threshold, CONSOLELOCK, display timeout (AC and DC), `PowerSettingAccessCheckEx` for each, overlay (Power mode), effective power mode, `DelayLockInterval`, idle time. | nothing |
| `write-test` | On the active plan: lid action to Do nothing, sleep timers to the user's value + 7 s (never shorter). Each write alone, then `PowerSetActiveScheme`, with three detectors running. Then restores and checks with `powercfg`. | real plan, restored |
| `copy-test [--keep]` | Duplicates the active plan, makes the copy active, runs change triggers on it (API and `powercfg`), tries to delete it while active, switches back, deletes it. `--keep` leaves the copy active so a person can look at Settings and Lenovo Vantage. | a copy only |
| `watch --seconds N` | Logs every change to the watched values for N seconds: setting notifications, registry notifications (HKLM plans key, HKCU `Control Panel\Desktop`), a 500 ms poll, effective power mode, suspend and resume. Ends with a list of values that differ from the start. Ctrl+C ends it early and still prints that list. | nothing |
| `hold [--sleep-dc S] [--minutes M] [--request R]` | Battery rule test. Makes a plan copy with battery sleep S seconds (default 60), makes it active, holds request R (`system+execution` default, `system`, `stes`, `none`) for M minutes (default 12), logs every 5 s, then switches back, deletes the copy and prints the Kernel-Power events of the test window. Refuses on AC unless `--allow-ac`. | a copy only |
| `restore` | Undoes what `out\journal.txt` lists: puts values back only if they still hold what the spike wrote, switches away from a spike copy, deletes copies. | as needed |
| `set-overlay <guid>` | Puts a Power mode overlay back with the undocumented `PowerSetActiveOverlayScheme`. Only for repair. | overlay |
| `limited <mode> ...` | Starts the mode with Explorer as parent, so it runs with the unelevated Medium token, waits, and prints the child's log. Use it from an elevated shell to test as a normal user. | as the mode |

`write-test`, `copy-test` and `hold` refuse to run elevated unless `--allow-elevated`: the point is to test a normal
user. For Haik's steps, run them from `spikes\morning-shell.cmd` opened in File Explorer (normal Medium token; see
`docs/phases/00-morning-steps.md`). From an elevated agent session, use `limited`.

They also refuse while `spikes\lid-sleep\out\journal.txt` exists or while the active plan is a spike copy (name
starts with "Wakeshot spike"): a copy of the other spike's copy would carry its test values.

## Safety

- `out\journal.txt` lists what the spike changed: a value before it is written, a copy right after it is made (its
  GUID does not exist before). A kill in that short gap leaves an unlisted, inactive copy named like the user's plan.
  A clean run deletes the journal. If a run is killed, `restore` puts things back.
- `restore` never switches to a plan that is gone or is a spike copy; it falls back to Balanced, then to any other
  plan, before it deletes the active copy.
- Values written to the real plan are never shorter than the user's own (lid Do nothing, timers + 7 s).
- `copy-test` and `hold` change only a plan copy, and every write names the copy. Ctrl+C or closing the window makes
  them switch back, delete the copy and remove the journal (the console handler gives them about 4.5 s on close).
  Sign-out and shutdown do not reach the handler (the exe loads user32), and a kill skips it: then the copy stays
  active until `restore`.

## Notes

- `wevtutil /f:text` prints local time with a `Z` suffix on this machine. Compare with the log's local time.
- The overlay GUID labels in the log come from community lists, not Microsoft docs.
