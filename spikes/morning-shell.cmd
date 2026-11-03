@echo off
rem Double-click this in File Explorer. Explorer starts it with the normal, unelevated token.
rem It opens PowerShell with the spike commands for docs\phases\00-morning-steps.md.
pwsh.exe -NoExit -NoLogo -NoProfile -Command ". '%~dp0morning-shell.ps1' %*"
