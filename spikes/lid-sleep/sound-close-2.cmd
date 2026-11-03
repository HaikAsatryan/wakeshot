@echo off
rem Card p1-lid-sound, close 2: DELL connected. Open it from File Explorer, which starts it unelevated.
rem At the close: our chime on the picked device, then the lock. 3 s later: the Phase 0 sound as a control.
rem Loopback listeners log what reaches each playback device.
"%~dp0target\release\lid-sleep-spike.exe" lid --sound chime --alias-after 3 --listen
pause
