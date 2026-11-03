@echo off
rem Card p1-lid-sound, close 1: laptop alone (DELL cable out). Open it from File Explorer, which starts it unelevated.
rem At the close: the Phase 0 sound and the lock 2 ms later, exactly as run I. 3 s later: our chime.
rem Loopback listeners log what reaches each playback device.
"%~dp0target\release\lid-sleep-spike.exe" lid --sound alias --chime-after 3 --listen
pause
