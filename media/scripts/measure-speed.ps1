<#
.SYNOPSIS
  Measures Wakeshot's speed numbers on the installed release build, by the method in docs/research/launch.md part 1
  ("Speed numbers: how p6-media measures each"). Writes one JSON file per mode; data/speed.json takes the results.

.DESCRIPTION
  Run each mode at a normal (Medium) PowerShell, with Wakeshot installed by its Setup.exe and running from the tray.
  Before idle and hold: no other agent, build or large program running, nobody using the laptop.

    -Mode snapshot -Out before.json    the power settings the e2e checks compare (read only)
    -Mode compare  -Before before.json -After after.json
    -Mode size     -Setup <path to Setup.exe>
    -Mode idle     -Minutes 10         nothing may hold during it
    -Mode hold     -Minutes 10         starts `wakeshot run -- ping -n 600 127.0.0.1` and samples tray and guard
    -Mode delays   -Minutes 30         then start and finish 5 short Claude Code turns by hand while it runs

  It changes no power setting itself; the hold mode makes Wakeshot write and restore its two settings, as in normal
  use, so take a snapshot before and compare after.
#>
param(
    [Parameter(Mandatory)] [ValidateSet('snapshot', 'compare', 'size', 'idle', 'hold', 'delays')] [string] $Mode,
    [int] $Minutes = 10,
    [string] $Out,
    [string] $Before,
    [string] $After,
    [string] $Setup
)

$ErrorActionPreference = 'Stop'
$install = Join-Path $env:LOCALAPPDATA 'HaikAsatryan.Wakeshot'
$dataDir = Join-Path $env:LOCALAPPDATA 'Wakeshot'
$log = Join-Path $dataDir 'logs\wakeshot.log'
$sessions = if ($env:CLAUDE_CONFIG_DIR) { Join-Path $env:CLAUDE_CONFIG_DIR 'sessions' } else { Join-Path $HOME '.claude\sessions' }

function Save($object, $default) {
    $path = if ($Out) { $Out } else { $default }
    $object | ConvertTo-Json -Depth 6 | Set-Content -Path $path -Encoding utf8
    Write-Host "wrote $path"
}

function Median([double[]] $values) {
    $sorted = $values | Sort-Object
    $n = $sorted.Count
    if ($n -eq 0) { return $null }
    if ($n % 2) { return $sorted[($n - 1) / 2] }
    return ($sorted[$n / 2 - 1] + $sorted[$n / 2]) / 2
}

function Build-Version {
    $exe = Get-ChildItem -Path $install -Recurse -Filter 'wakeshot-tray.exe' -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $exe) { throw "no wakeshot-tray.exe under ${install}: install the release build first" }
    return @{ path = $exe.FullName; version = $exe.VersionInfo.ProductVersion }
}

# The tray is wakeshot-tray.exe without --guard; the guard is the same exe with --guard (crates/wakeshot-tray/src/guard.rs).
function Wakeshot-Processes {
    Get-CimInstance Win32_Process -Filter "Name = 'wakeshot-tray.exe'" | ForEach-Object {
        [pscustomobject]@{ Id = [int]$_.ProcessId; Guard = ($_.CommandLine -match '--guard') }
    }
}

# Other work that would spoil an idle number: builds, agents, the e2e suites.
function Assert-Quiet {
    $busy = Get-Process -ErrorAction SilentlyContinue | Where-Object { $_.ProcessName -match '^(cargo|rustc|link|claude|node|msbuild|dotnet)$' }
    if ($busy) { throw "other work is running: $(($busy | ForEach-Object { "$($_.ProcessName) $($_.Id)" }) -join ', ')" }
}

function Sample([int[]] $ids, [int] $minutes) {
    $start = @{}
    foreach ($id in $ids) { $start[$id] = (Get-Process -Id $id).TotalProcessorTime.TotalMilliseconds }
    $rows = @{}
    foreach ($id in $ids) { $rows[$id] = [System.Collections.Generic.List[object]]::new() }
    $end = (Get-Date).AddMinutes($minutes)
    while ((Get-Date) -lt $end) {
        foreach ($id in $ids) {
            $p = Get-Process -Id $id -ErrorAction SilentlyContinue
            if ($p) { $rows[$id].Add(@{ ws = $p.WorkingSet64; priv = $p.PrivateMemorySize64 }) }
        }
        Start-Sleep -Seconds 10
    }
    $result = @{}
    foreach ($id in $ids) {
        $p = Get-Process -Id $id -ErrorAction SilentlyContinue
        $cpu = if ($p) { $p.TotalProcessorTime.TotalMilliseconds - $start[$id] } else { $null }
        $ws = [double[]]($rows[$id] | ForEach-Object { $_.ws })
        $priv = [double[]]($rows[$id] | ForEach-Object { $_.priv })
        $result["$id"] = @{
            samples = $rows[$id].Count
            workingSetMedianMb = [math]::Round((Median $ws) / 1MB, 2)
            workingSetMaxMb = [math]::Round(($ws | Measure-Object -Maximum).Maximum / 1MB, 2)
            privateMedianMb = [math]::Round((Median $priv) / 1MB, 2)
            privateMaxMb = [math]::Round(($priv | Measure-Object -Maximum).Maximum / 1MB, 2)
            cpuMs = $cpu
            cpuPercentOfOneCore = if ($null -ne $cpu) { [math]::Round(100 * $cpu / ($minutes * 60000), 4) } else { $null }
        }
    }
    return $result
}

function Read-Plan {
    $active = (powercfg /getactivescheme) -replace '.*GUID: ([0-9a-f-]+).*', '$1'
    $read = {
        param($sub, $setting)
        $text = powercfg /q $active $sub $setting
        $ac = ($text | Select-String 'Current AC Power Setting Index: (0x[0-9a-f]+)').Matches[0].Groups[1].Value
        $dc = ($text | Select-String 'Current DC Power Setting Index: (0x[0-9a-f]+)').Matches[0].Groups[1].Value
        @{ ac = [Convert]::ToInt64($ac, 16); dc = [Convert]::ToInt64($dc, 16) }
    }
    $runOnce = Get-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\RunOnce' -ErrorAction SilentlyContinue
    return [ordered]@{
        activePlan = $active
        lidAction = & $read SUB_BUTTONS LIDACTION
        sleepAfter = & $read SUB_SLEEP STANDBYIDLE
        screenOff = & $read SUB_VIDEO VIDEOIDLE
        energySaverDc = & $read SUB_ENERGYSAVER ESBATTTHRESHOLD
        plans = @(powercfg /l | Select-String 'GUID:' | ForEach-Object { $_.Line.Trim() })
        runOnce = if ($runOnce) { @($runOnce.PSObject.Properties | Where-Object { $_.Name -notlike 'PS*' } | ForEach-Object { "$($_.Name)=$($_.Value)" }) } else { @() }
    }
}

switch ($Mode) {
    'snapshot' {
        Save (Read-Plan) 'power-snapshot.json'
    }
    'compare' {
        $a = Get-Content $Before -Raw
        $b = if ($After) { Get-Content $After -Raw } else { (Read-Plan) | ConvertTo-Json -Depth 6 }
        if ((($a | ConvertFrom-Json) | ConvertTo-Json -Depth 6) -eq (($b | ConvertFrom-Json) | ConvertTo-Json -Depth 6)) {
            Write-Host 'SAME: the power settings equal the snapshot'
        } else {
            Write-Host 'DIFF: the power settings differ from the snapshot'
            Write-Host "before: $a"
            Write-Host "after:  $b"
            exit 1
        }
    }
    'size' {
        $build = Build-Version
        $dir = Split-Path $build.path
        $files = @{}
        foreach ($name in 'wakeshot-tray.exe', 'wakeshot.exe') {
            $f = Join-Path $dir $name
            if (Test-Path $f) { $files[$name] = (Get-Item $f).Length }
        }
        if ($Setup) { $files['Setup.exe'] = (Get-Item $Setup).Length }
        $total = (Get-ChildItem -Path $install -Recurse -File | Measure-Object -Property Length -Sum).Sum
        Save @{ mode = 'size'; build = $build.version; date = (Get-Date).ToString('yyyy-MM-dd'); bytes = $files; installFolderBytes = $total } 'speed-size.json'
    }
    'idle' {
        Assert-Quiet
        $build = Build-Version
        $tray = Wakeshot-Processes | Where-Object { -not $_.Guard }
        if (-not $tray) { throw 'Wakeshot is not running' }
        if (Wakeshot-Processes | Where-Object { $_.Guard }) { throw 'a guard runs, so something holds or a setting is overridden: wait for idle' }
        Write-Host "sampling tray $($tray.Id) for $Minutes minutes; leave the laptop alone"
        $r = Sample @($tray.Id) $Minutes
        Save @{ mode = 'idle'; build = $build.version; date = (Get-Date).ToString('yyyy-MM-dd'); minutes = $Minutes; tray = $r["$($tray.Id)"] } 'speed-idle.json'
    }
    'hold' {
        Assert-Quiet
        $build = Build-Version
        $tray = Wakeshot-Processes | Where-Object { -not $_.Guard }
        if (-not $tray) { throw 'Wakeshot is not running' }
        $cli = Join-Path (Split-Path $build.path) 'wakeshot.exe'
        # A fixed load: the same command every run (launch.md part 1).
        $run = Start-Process -FilePath $cli -ArgumentList 'run', '--', 'ping', '-n', '600', '127.0.0.1' -WindowStyle Hidden -PassThru
        Start-Sleep -Seconds 15
        $guard = Wakeshot-Processes | Where-Object { $_.Guard }
        $ids = @($tray.Id) + @($guard | ForEach-Object { $_.Id })
        Write-Host "hold by wakeshot run (pid $($run.Id)); sampling $($ids -join ', ') for $Minutes minutes"
        $r = Sample $ids $Minutes
        # Stop only the process this script started, by its id.
        if (-not $run.HasExited) { Stop-Process -Id $run.Id }
        Save @{ mode = 'hold'; build = $build.version; date = (Get-Date).ToString('yyyy-MM-dd'); minutes = $Minutes; tray = $r["$($tray.Id)"]; guard = @($guard | ForEach-Object { $r["$($_.Id)"] }) } 'speed-hold.json'
    }
    'delays' {
        $build = Build-Version
        Write-Host "watching $sessions for $Minutes minutes: start and finish 5 short Claude Code turns now"
        $seen = @{}
        $events = [System.Collections.Generic.List[object]]::new()
        $startedAt = Get-Date
        $end = $startedAt.AddMinutes($Minutes)
        while ((Get-Date) -lt $end) {
            # Only the status files; never the .key files next to them (CLAUDE.md).
            foreach ($f in Get-ChildItem -Path $sessions -Filter '*.json' -File -ErrorAction SilentlyContinue) {
                try { $j = Get-Content $f.FullName -Raw | ConvertFrom-Json } catch { continue }
                if (-not $j.statusUpdatedAt) { continue }
                $key = "$($f.Name)|$($j.statusUpdatedAt)"
                if ($seen[$key]) { continue }
                $seen[$key] = $true
                if ($j.status -in 'busy', 'idle') { $events.Add(@{ status = $j.status; at = [int64]$j.statusUpdatedAt }) }
            }
            Start-Sleep -Milliseconds 100
        }
        # Wakeshot's own lines (crates/wakeshot-core/src/core.rs): "Holding starts: ..." and "Holding ends; ...",
        # each after an ISO time with its offset.
        $holds = Get-Content $log | ForEach-Object {
            if ($_ -match '^(\S+) .*core: Holding (starts|ends)') {
                $t = [DateTimeOffset]::Parse($Matches[1]).ToUnixTimeMilliseconds()
                if ($t -ge ([DateTimeOffset]$startedAt).ToUnixTimeMilliseconds()) { @{ kind = $Matches[2]; at = $t } }
            }
        }
        $pairs = foreach ($e in $events) {
            $want = if ($e.status -eq 'busy') { 'starts' } else { 'ends' }
            $h = $holds | Where-Object { $_.kind -eq $want -and $_.at -ge $e.at } | Sort-Object { $_.at } | Select-Object -First 1
            if ($h) { @{ status = $e.status; delayMs = $h.at - $e.at } }
        }
        $startMs = [double[]]($pairs | Where-Object { $_.status -eq 'busy' } | ForEach-Object { $_.delayMs })
        $endMs = [double[]]($pairs | Where-Object { $_.status -eq 'idle' } | ForEach-Object { $_.delayMs })
        Save @{
            mode = 'delays'; build = $build.version; date = (Get-Date).ToString('yyyy-MM-dd')
            startDelayMs = @{ runs = $startMs.Count; median = (Median $startMs); max = ($startMs | Measure-Object -Maximum).Maximum }
            endDelayMs = @{ runs = $endMs.Count; median = (Median $endMs); max = ($endMs | Measure-Object -Maximum).Maximum }
            pairs = @($pairs)
        } 'speed-delays.json'
    }
}
