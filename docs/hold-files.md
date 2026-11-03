# Hold files

Any script or tool can keep the PC awake while it runs: it writes a small JSON file to Wakeshot's `holds` folder.
The PC stays awake while the process named in the file runs, then sleeps as usual. `wakeshot run -- <command>` writes
such a file for you, so use it when you can. Write your own file when the work is not one command.

## Where

`%LocalAppData%\Wakeshot\holds\`, for example `C:\Users\<you>\AppData\Local\Wakeshot\holds\`. Create the folder if
it does not exist.

One file per hold. The name must end in `.json`; Wakeshot never reads a file with any other name. Two names are
Wakeshot's own:

- `keep-awake.json`: Keep awake from the menu and `wakeshot keep-awake`.
- `run-<digits>.json`: `wakeshot run`.

Give your file any other name, such as `nightly-backup.json`. Case does not matter, as in all Windows file names.

## Format

```json
{ "version": 1, "pid": 4242, "procStart": "134381211391959336",
  "reason": "nightly backup", "expires": "2026-11-03T18:00:00Z", "screen": false }
```

The file is UTF-8 JSON. Only the first JSON value is read; anything after it is ignored. A UTF-8 byte order mark at
the start is fine. Fields not listed here are ignored.

| Field | Required | Meaning |
|---|---|---|
| `version` | yes | `1`. |
| `pid` | yes | The process the hold belongs to. The hold lasts while it runs. |
| `procStart` | no | When that process started: a Windows FILETIME, as a string of digits in quotes. |
| `reason` | no | A short label for the tooltip and the log. |
| `expires` | no | When the hold ends, even if the process still runs. |
| `screen` | no | `true` keeps the screen on too. Counts only in `keep-awake.json`. |

## Rules

- `pid` is required. Name a process that lives as long as the work, usually the script itself.
- `procStart` is a string of digits such as `"134381211391959336"`, never a JSON number. With it, the hold counts
  only while that exact process runs. In PowerShell it is `(Get-Process -Id $PID).StartTime.ToFileTimeUtc()`.
- Without `procStart`, the process counts only if it started at or before the file's last write time. Windows reuses
  pids, and a process that started after the file was written cannot be the one the file means. So write the file
  after the process has started.
- `reason` is cut to 60 characters, and line breaks and other control characters become spaces. Write a short label
  such as the tool's name. Never write a full command line: it may hold passwords or tokens, and the reason shows in
  the tooltip and the log.
- `expires` is an RFC 3339 time. Write it in UTC with `Z`, such as `2026-11-03T18:00:00Z`. An offset such as
  `+04:00` and a fraction of a second also work. A time with no zone, such as `2026-11-03T18:00:00` (what
  `Get-Date -Format s` gives), makes the whole file bad.
- `screen` counts only in `keep-awake.json` and is ignored in any other file: only the user turns the screen on,
  through Keep screen on.
- When the process is gone, the hold ends and Wakeshot deletes the file. Deleting it yourself when the work is done
  ends the hold at once.
- A bad file (not JSON, no `pid`, a bad `expires`) is ignored and logged: it does not hold. Wakeshot reads it once
  more after a short wait first, in case it was still being written. A file with a newer `version` than this
  Wakeshot knows is ignored and logged too. `wakeshot log` opens the log.
- Write the file under a temp name that does not end in `.json`, then rename it to its final name. Then Wakeshot
  never sees a half-written file.
- When Automatic is off, hold files do not keep the PC awake. Only Keep awake does.

## PowerShell example

For PowerShell 7 and Windows PowerShell 5.1. The hold lasts while this script runs, at most 4 hours.

```powershell
$holds = Join-Path $env:LOCALAPPDATA 'Wakeshot\holds'
$hold = Join-Path $holds 'nightly-backup.json'
$temp = Join-Path $holds 'nightly-backup.tmp'

New-Item -ItemType Directory -Force $holds | Out-Null
$file = [ordered]@{
    version   = 1
    pid       = $PID
    procStart = [string](Get-Process -Id $PID).StartTime.ToFileTimeUtc()
    reason    = 'nightly backup'
    expires   = (Get-Date).ToUniversalTime().AddHours(4).ToString('o')
}
[IO.File]::WriteAllText($temp, ($file | ConvertTo-Json -Compress))
Move-Item -Force $temp $hold

try {
    # The long work goes here.
} finally {
    Remove-Item $hold -ErrorAction SilentlyContinue
}
```

## cmd example

A batch file that holds while it runs. cmd cannot tell its own pid, so it asks PowerShell for the pid of its parent,
which is this cmd.exe. cmd writes text in the console code page, so keep `reason` to plain ASCII here.

```bat
@echo off
set "HOLDS=%LOCALAPPDATA%\Wakeshot\holds"
if not exist "%HOLDS%" mkdir "%HOLDS%"

powershell -NoProfile -Command "(Get-CimInstance Win32_Process -Filter ('ProcessId=' + $PID)).ParentProcessId" > "%TEMP%\nightly-backup.pid"
set /p HOLDPID=<"%TEMP%\nightly-backup.pid"
del "%TEMP%\nightly-backup.pid"

> "%HOLDS%\nightly-backup.tmp" echo {"version": 1, "pid": %HOLDPID%, "reason": "nightly backup"}
move /y "%HOLDS%\nightly-backup.tmp" "%HOLDS%\nightly-backup.json" > nul

rem The long work goes here.

del "%HOLDS%\nightly-backup.json"
```

This file has no `procStart`, so the pid-only rule above applies: cmd.exe started before the file was written, so it
counts.
