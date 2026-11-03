# Shell for Haik's morning steps (docs\phases\00-morning-steps.md).
# Start it with morning-shell.cmd from File Explorer: Explorer gives its normal, unelevated token,
# while Windows Terminal on this laptop opens elevated (profiles.defaults.elevate is true).
# Dot-sourced by morning-shell.cmd, so the functions stay defined in the window.
param([switch]$SelfTest)

$WakeshotSpikes = 'C:\Users\haika\code\wakeshot\spikes'
$WakeshotPowerExe = "$WakeshotSpikes\power-settings\target\release\power-settings-spike.exe"
$WakeshotLidExe = "$WakeshotSpikes\lid-sleep\target\release\lid-sleep-spike.exe"
$WakeshotTrayExe = "$WakeshotSpikes\tray\target\x86_64-pc-windows-msvc\release\tray-spike.exe"
$WakeshotBalanced = '381b4222-f694-41f0-9685-ff5bb260df2e'

function spike { & $WakeshotPowerExe @args }
function lidspike { & $WakeshotLidExe @args }
# A window-less GUI exe: PowerShell starts it and returns at once.
function trayspike { & $WakeshotTrayExe @args }

function restoreall {
    lidspike restore
    spike restore
}

function Get-WakeshotIntegrity {
    $line = whoami /groups | Select-String 'Mandatory Level' | Select-Object -First 1
    if ($line -match 'Mandatory Label\\(\w+) Mandatory Level') { return $Matches[1] }
    return 'unknown'
}

# One powercfg value as [ac, dc]; $null when the read fails.
function Get-WakeshotValue([string]$sub, [string]$setting) {
    $out = powercfg /qh SCHEME_CURRENT $sub $setting 2>&1
    $ac = $out | Select-String 'Current AC Power Setting Index: (0x[0-9a-fA-F]+)'
    $dc = $out | Select-String 'Current DC Power Setting Index: (0x[0-9a-fA-F]+)'
    if (-not $ac -or -not $dc) { return $null }
    return @([Convert]::ToInt64($ac.Matches[0].Groups[1].Value, 16), [Convert]::ToInt64($dc.Matches[0].Groups[1].Value, 16))
}

# Haik's own values, read on 2026-11-03 at 02:04, 02:31, 03:04 and 03:30.
function checkpower {
    $bad = 0
    $plans = @(powercfg /list | Select-String 'GUID: ([0-9a-f-]{36})\s+\((.*)\)(\s*\*)?' | ForEach-Object {
        [pscustomobject]@{ Guid = $_.Matches[0].Groups[1].Value; Name = $_.Matches[0].Groups[2].Value; Active = [bool]$_.Matches[0].Groups[3].Value }
    })
    foreach ($p in $plans) {
        $mark = if ($p.Active) { 'active' } else { '' }
        if ($p.Guid -eq $WakeshotBalanced) {
            Write-Host ("  plan {0} ({1}) {2}" -f $p.Guid, $p.Name, $mark)
        } else {
            Write-Host ("  plan {0} ({1}) {2}  <- NOT Haik's plan" -f $p.Guid, $p.Name, $mark) -ForegroundColor Red
            $bad++
        }
    }
    $active = $plans | Where-Object Active | Select-Object -First 1
    if (-not $active -or $active.Guid -ne $WakeshotBalanced) {
        Write-Host '  active plan is NOT Balanced' -ForegroundColor Red
        $bad++
    }
    $expect = @(
        @('lid action (1 = Sleep)', 'SUB_BUTTONS', 'LIDACTION', 1, 1),
        @('sleep after (s)', 'SUB_SLEEP', 'STANDBYIDLE', 10800, 600),
        @('wake timers (0 = Disable)', 'SUB_SLEEP', 'RTCWAKE', 0, 0),
        @('screen off after (s)', 'SUB_VIDEO', 'VIDEOIDLE', 300, 180),
        @('energy saver at (%)', 'SUB_ENERGYSAVER', 'ESBATTTHRESHOLD', 0, 30),
        @('sign-in after sleep (1 = on)', 'SUB_NONE', 'CONSOLELOCK', 1, 1)
    )
    foreach ($e in $expect) {
        $v = Get-WakeshotValue $e[1] $e[2]
        if ($null -eq $v) {
            Write-Host ("  {0}: could not read" -f $e[0]) -ForegroundColor Red
            $bad++
        } elseif ($v[0] -eq $e[3] -and $v[1] -eq $e[4]) {
            Write-Host ("  {0}: AC {1}, battery {2}  OK" -f $e[0], $v[0], $v[1])
        } else {
            Write-Host ("  {0}: AC {1}, battery {2}  DIFFERENT, Haik's is AC {3}, battery {4}" -f $e[0], $v[0], $v[1], $e[3], $e[4]) -ForegroundColor Red
            $bad++
        }
    }
    $overlay = (Get-ItemProperty 'HKLM:\SYSTEM\CurrentControlSet\Control\Power\User\PowerSchemes' -ErrorAction SilentlyContinue).ActiveOverlayAcPowerScheme
    if ($overlay -eq 'ded574b5-45a0-4f42-8737-46345c09c238') {
        Write-Host '  Power mode on AC (overlay): Haik''s  OK'
    } else {
        Write-Host ("  Power mode on AC (overlay): {0}  DIFFERENT, Haik's is ded574b5-45a0-4f42-8737-46345c09c238" -f $overlay) -ForegroundColor Red
        $bad++
    }
    foreach ($j in "$WakeshotSpikes\power-settings\out\journal.txt", "$WakeshotSpikes\lid-sleep\out\journal.txt") {
        if (Test-Path $j) {
            Write-Host "  journal left: $j  -> run restoreall" -ForegroundColor Red
            $bad++
        }
    }
    if ($bad -eq 0) {
        Write-Host 'checkpower: ALL OK, your settings are as before.' -ForegroundColor Green
    } else {
        Write-Host "checkpower: $bad DIFFERENT. Run restoreall, then checkpower again. If still different: the restore block at the top of docs\phases\00-morning-steps.md." -ForegroundColor Red
    }
    # A global, not a return value: typed at the prompt, a returned count prints as a stray last line.
    $global:WakeshotCheckBad = $bad
}

# For the log, so the host is known and not guessed. WT_SESSION cannot show it: when Windows Terminal takes over a
# console that Explorer started, the shell already runs, so Terminal cannot add the variable to it.
function Get-WakeshotConsoleHost {
    try {
        if (-not ('Wakeshot.Con' -as [type])) {
            Add-Type -Namespace Wakeshot -Name Con -MemberDefinition @'
[DllImport("kernel32.dll")] public static extern IntPtr GetConsoleWindow();
[DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetClassName(IntPtr h, System.Text.StringBuilder s, int n);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
[DllImport("user32.dll")] public static extern IntPtr GetWindow(IntPtr h, uint cmd);
'@
        }
        $h = [Wakeshot.Con]::GetConsoleWindow()
        $class = New-Object System.Text.StringBuilder 256
        $null = [Wakeshot.Con]::GetClassName($h, $class, 256)
        [uint32]$winPid = 0
        $null = [Wakeshot.Con]::GetWindowThreadProcessId($h, [ref]$winPid)
        $owner = [Wakeshot.Con]::GetWindow($h, 4)
        [uint32]$ownerPid = 0
        if ($owner -ne [IntPtr]::Zero) { $null = [Wakeshot.Con]::GetWindowThreadProcessId($owner, [ref]$ownerPid) }
        $winName = (Get-Process -Id $winPid -ErrorAction SilentlyContinue).ProcessName
        $ownerName = if ($ownerPid) { (Get-Process -Id $ownerPid -ErrorAction SilentlyContinue).ProcessName } else { 'none' }
        return "window class $class, window process $winName/$winPid, owner $ownerName/$ownerPid"
    } catch {
        return "unknown ($($_.Exception.Message))"
    }
}

$WakeshotIntegrity = Get-WakeshotIntegrity
$WakeshotAdmin = (New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
# The parent chain shows how the shell was started (Explorer, a terminal, an agent).
$chain = @()
$p = $PID
for ($i = 0; $i -lt 4 -and $p; $i++) {
    $proc = Get-CimInstance Win32_Process -Filter "ProcessId = $p" -ErrorAction SilentlyContinue
    if (-not $proc) { break }
    $chain += "$($proc.Name)/$($proc.ProcessId)"
    $p = $proc.ParentProcessId
}
Add-Content -Path "$WakeshotSpikes\morning-shell.log" -Value ("{0} integrity {1} admin-enabled {2} console {3} WT_SESSION {4} selftest {5} chain {6}" -f (Get-Date -Format 'yyyy-MM-dd HH:mm:ss'), $WakeshotIntegrity, $WakeshotAdmin, (Get-WakeshotConsoleHost), [bool]$env:WT_SESSION, [bool]$SelfTest, ($chain -join ' <- '))

$Host.UI.RawUI.WindowTitle = "Wakeshot morning shell ($WakeshotIntegrity)"
Write-Host ''
if ($WakeshotIntegrity -eq 'Medium' -and -not $WakeshotAdmin) {
    Write-Host 'Token: Medium Mandatory Level, not elevated. Good: this is a normal user shell.' -ForegroundColor Green
} else {
    Write-Host "STOP: this shell runs at $WakeshotIntegrity (admin enabled: $WakeshotAdmin)." -ForegroundColor Red
    Write-Host 'Close this window. Open File Explorer, go to C:\Users\haika\code\wakeshot\spikes and double-click morning-shell.cmd.' -ForegroundColor Red
}
Write-Host ''
Write-Host 'Your power settings now:'
checkpower
Write-Host ''
Write-Host 'Commands: spike, lidspike, trayspike, restoreall, checkpower. Steps: docs\phases\00-morning-steps.md'

if ($SelfTest) {
    # Starts each tool the way the morning steps do, in modes that write no setting.
    $out = "$WakeshotSpikes\morning-shell-selftest.log"
    "selftest {0}: shell pid {1}, integrity {2}, admin-enabled {3}" -f (Get-Date -Format 'HH:mm:ss'), $PID, $WakeshotIntegrity, $WakeshotAdmin | Set-Content $out
    # Collect all output first: Select-Object -First on a live pipeline stops the exe early.
    $o = spike read
    ($o | Select-String 'mode read' | Select-Object -First 1).Line | Add-Content $out
    $o = lidspike read
    ($o | Select-String 'mode read|SeShutdownPrivilege' | ForEach-Object { $_.Line }) | Add-Content $out
    $trayLog = "$WakeshotSpikes\tray\out\log-token.txt"
    Remove-Item $trayLog -ErrorAction SilentlyContinue
    trayspike --token
    for ($i = 0; $i -lt 50 -and -not (Test-Path $trayLog); $i++) { Start-Sleep -Milliseconds 100 }
    Start-Sleep -Milliseconds 300
    (Get-Content $trayLog -ErrorAction SilentlyContinue | Select-String 'token:' | Select-Object -First 1).Line | Add-Content $out
    checkpower 6>$null
    "checkpower: $global:WakeshotCheckBad different" | Add-Content $out
    [Environment]::Exit(0)
}
