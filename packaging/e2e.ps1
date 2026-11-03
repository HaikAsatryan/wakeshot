#Requires -Version 7

<#
.SYNOPSIS
Installs, updates and uninstalls Wakeshot for real on this PC and checks that the real hooks put back a journaled
power value each time, write and remove the Run value, the RunOnce value and the Path entry, and leave the PC as it
was. One line per check; exit 0 when all pass, 1 when any fails, 2 when it refuses to run.

.DESCRIPTION
The steps of docs/tasks/p6-install-e2e.md:

1. BEFORE: the Run and RunOnce keys, the user Path (type and bytes), the active plan and its lid, sleep and energy
   saver values. A data folder %LocalAppData%\Wakeshot is moved into -WorkDir, since the uninstall deletes it.
2. A copy of the active plan named "Wakeshot test e2e", never made active. Every journal below names only the copy.
3. Packs 0.1.0 and 0.1.1 of the exes in -ExeDir with pack.ps1 (the build is the caller's).
4. Setup.exe --silent of 0.1.0, then the install checks of smoke-test.ps1.
5. A hold: the copy's DC lid action set to Do nothing and a journal that owns it, its owner a process this script
   started and ended. Then Setup.exe --silent of 0.1.0 over the install, as winget upgrade runs it: the install hook
   must put the value back.
6. The same hold; 0.1.1 staged in packages\ and applied with Update.exe apply --silent --norestart (research.md,
   "Velopack checks for Wakeshot", point 8): the updated hook must put it back, and current must keep its path.
7. The same hold and a test RunOnce value; Update.exe --uninstall --silent: the uninstall hook must put it back and
   remove the Run, RunOnce and Path values; the data, install and temp folders, the uninstall key and the shortcut go.
8. AFTER: the copy deleted, the data folder put back, every BEFORE value compared, no process this script started
   left.

Whatever stops the run, the end uninstalls, removes the test RunOnce value, puts back the user Path bytes, the active
plan and its values if they differ from BEFORE, deletes the copy and moves the data folder back. A hold never names
the real plan, so no step can change it unless a hook writes the wrong plan.

Run it unelevated (Medium), as users run Wakeshot: an elevated run installs with a token no user has. -AllowElevated
is for a CI runner, where every process is elevated. It refuses while Wakeshot is installed, a journal exists, or a
"Wakeshot test e2e" plan is active.

Where the terminal opens elevated (this laptop's does), a .cmd opened with explorer.exe runs at Medium (research.md,
"Spike launch and stop checks"); the second example is such a .cmd. Write the exit code as `(echo %ERRORLEVEL%)`: in
`echo %ERRORLEVEL%> file`, cmd reads `0>` as a redirect of handle 0 and writes nothing.

.EXAMPLE
cargo build --release --locked --target x86_64-pc-windows-msvc -p wakeshot-tray -p wakeshot-cli
./packaging/e2e.ps1 -ExeDir target/x86_64-pc-windows-msvc/release -WorkDir <new folder>

.EXAMPLE
pwsh.exe -NoProfile -File <repo>\packaging\e2e.ps1 -ExeDir <exes> -WorkDir <new folder> > <log> 2>&1
(echo %ERRORLEVEL%)> <exit file>
#>
[CmdletBinding()]
param(
    # The folder that holds the x64 release builds of wakeshot-tray.exe and wakeshot.exe.
    [Parameter(Mandatory)]
    [string] $ExeDir,

    # Packs, saved logs, BEFORE and the moved data folder go here. It must be empty or not exist yet.
    [Parameter(Mandatory)]
    [string] $WorkDir,

    [string] $Icon = (Join-Path $PSScriptRoot '..\assets\wakeshot.ico'),

    # The vpk command; its version must match the velopack crate (1.2.161).
    [string] $Vpk = 'vpk',

    [switch] $AllowElevated,

    # Each install, apply and uninstall may take this long. Velopack's own hook limits are 30 s and 60 s.
    [int] $TimeoutSeconds = 120
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false

$PackId = 'HaikAsatryan.Wakeshot'
$CopyName = 'Wakeshot test e2e'
$OldVersion = '0.1.0'
$NewVersion = '0.1.1'
$LocalAppData = [Environment]::GetFolderPath('LocalApplicationData')
$Root = Join-Path $LocalAppData $PackId
$Current = Join-Path $Root 'current'
$TrayExe = Join-Path $Current 'wakeshot-tray.exe'
$UpdateExe = Join-Path $Root 'Update.exe'
$Packages = Join-Path $Root 'packages'
$DataDir = Join-Path $LocalAppData 'Wakeshot'
$Journal = Join-Path $DataDir 'journal.json'
$HookLog = Join-Path $DataDir 'logs\wakeshot.log'
$Shortcut = Join-Path ([Environment]::GetFolderPath('Programs')) 'Wakeshot.lnk'
$VeloTemp = Join-Path ([IO.Path]::GetTempPath()) "velopack_$PackId"
$VelopackLogs = Join-Path $LocalAppData 'velopack'
$UninstallSub = "Software\Microsoft\Windows\CurrentVersion\Uninstall\$PackId"
$RunSub = 'Software\Microsoft\Windows\CurrentVersion\Run'
$RunOnceSub = 'Software\Microsoft\Windows\CurrentVersion\RunOnce'
$RunName = 'Wakeshot'
$RunOnceName = 'WakeshotRestore'
$EnvSub = 'Environment'
# What the install hook writes (06-ship.md, "Package and install values") and what the tray writes while it holds
# (power/journal.rs).
$RunCommand = "`"$TrayExe`""
$RunOnceCommand = "`"$TrayExe`" --restore"
$GuidPattern = '[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}'

# RegQueryValueExW gives the bytes as stored; the .NET registry API trims trailing NULs, which would hide a change.
if (-not ('WakeshotE2e.Reg' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;

namespace WakeshotE2e {
    public static class Reg {
        [DllImport("advapi32.dll", CharSet = CharSet.Unicode)]
        private static extern int RegOpenKeyExW(IntPtr key, string subKey, uint options, uint access, out IntPtr result);

        [DllImport("advapi32.dll", CharSet = CharSet.Unicode)]
        private static extern int RegQueryValueExW(IntPtr key, string name, IntPtr reserved, out uint type, byte[] data, ref uint size);

        [DllImport("advapi32.dll", CharSet = CharSet.Unicode)]
        private static extern int RegSetValueExW(IntPtr key, string name, uint reserved, uint type, byte[] data, uint size);

        [DllImport("advapi32.dll", CharSet = CharSet.Unicode)]
        private static extern int RegDeleteValueW(IntPtr key, string name);

        [DllImport("advapi32.dll")]
        private static extern int RegCloseKey(IntPtr key);

        [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern IntPtr SendMessageTimeoutW(IntPtr window, uint message, UIntPtr wParam, string lParam, uint flags, uint timeout, out UIntPtr result);

        private const int NotFound = 2;
        private const int MoreData = 234;
        private const uint QueryValue = 0x0001;
        private const uint SetValue = 0x0002;

        // HKEY_CURRENT_USER is sign-extended on 64-bit Windows, as the SDK headers define it.
        private static readonly IntPtr Hkcu = new IntPtr(unchecked((int)0x80000001));

        private static IntPtr Open(string subKey, uint access) {
            IntPtr key;
            int rc = RegOpenKeyExW(Hkcu, subKey, 0, access, out key);
            if (rc == NotFound) return IntPtr.Zero;
            if (rc != 0) throw new Win32Exception(rc);
            return key;
        }

        // null: the key or the value is missing.
        public static byte[] Read(string subKey, string name, out uint type) {
            type = 0;
            IntPtr key = Open(subKey, QueryValue);
            if (key == IntPtr.Zero) return null;
            try {
                while (true) {
                    uint size = 0;
                    int rc = RegQueryValueExW(key, name, IntPtr.Zero, out type, null, ref size);
                    if (rc == NotFound) return null;
                    if (rc != 0) throw new Win32Exception(rc);
                    byte[] data = new byte[size];
                    rc = RegQueryValueExW(key, name, IntPtr.Zero, out type, data, ref size);
                    // The value grew between the two calls.
                    if (rc == MoreData) continue;
                    if (rc != 0) throw new Win32Exception(rc);
                    Array.Resize(ref data, (int)size);
                    return data;
                }
            } finally {
                RegCloseKey(key);
            }
        }

        public static string Raw(string subKey, string name) {
            uint type;
            byte[] data = Read(subKey, name, out type);
            if (data == null) return "missing";
            return "type " + type + ", " + data.Length + " bytes: " + BitConverter.ToString(data);
        }

        public static void Write(string subKey, string name, uint type, byte[] data) {
            IntPtr key = Open(subKey, SetValue);
            if (key == IntPtr.Zero) throw new Win32Exception(NotFound);
            try {
                int rc = RegSetValueExW(key, name, 0, type, data, (uint)data.Length);
                if (rc != 0) throw new Win32Exception(rc);
            } finally {
                RegCloseKey(key);
            }
        }

        public static void Delete(string subKey, string name) {
            IntPtr key = Open(subKey, SetValue);
            if (key == IntPtr.Zero) return;
            try {
                int rc = RegDeleteValueW(key, name);
                if (rc != 0 && rc != NotFound) throw new Win32Exception(rc);
            } finally {
                RegCloseKey(key);
            }
        }

        // What the install code sends after a Path change, so running programs read the new value.
        public static bool Broadcast() {
            UIntPtr result;
            IntPtr ok = SendMessageTimeoutW(new IntPtr(0xffff), 0x001A, UIntPtr.Zero, "Environment", 0x0002, 1000, out result);
            return ok != IntPtr.Zero;
        }
    }
}
'@
}

$script:Failed = 0
$script:Started = [Collections.Generic.List[object]]::new()
$script:Times = [ordered]@{}

function Say([string] $Text) { Write-Host $Text }

function Pass([string] $Check) { Say "PASS  $Check" }

function Fail([string] $Check, [string] $Why) {
    Say "FAIL  $Check`: $Why"
    $script:Failed++
}

function Test-Check([string] $Check, [bool] $Ok, [string] $Why) {
    if ($Ok) { Pass $Check } else { Fail $Check $Why }
}

function Step([string] $Text) { Say "==== $((Get-Date).ToString('HH:mm:ss')) $Text" }

function Get-Hash([string] $Text) {
    [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($Text))).Substring(0, 16)
}

function Get-RegValue([string] $Sub, [string] $Name) {
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($Sub)
    if ($null -eq $key) { return $null }
    try { $key.GetValue($Name, $null, 'DoNotExpandEnvironmentNames') } finally { $key.Dispose() }
}

function Test-RegKey([string] $Sub) {
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($Sub)
    if ($null -eq $key) { return $false }
    $key.Dispose()
    $true
}

function Get-KeyState([string] $Sub) {
    $state = [ordered]@{}
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($Sub)
    if ($null -eq $key) { return $state }
    try { $names = @($key.GetValueNames() | Sort-Object) } finally { $key.Dispose() }
    foreach ($name in $names) { $state[$name] = [WakeshotE2e.Reg]::Raw($Sub, $name) }
    $state
}

function Compare-KeyState($Want, $Got) {
    $names = @(@($Want.Keys) + @($Got.Keys) | Sort-Object -Unique)
    @(foreach ($name in $names) {
            $a = if ($Want.Contains($name)) { $Want[$name] } else { 'missing' }
            $b = if ($Got.Contains($name)) { $Got[$name] } else { 'missing' }
            if ($a -cne $b) { "'$name' was $(Get-Hash $a), is $(Get-Hash $b)" }
        })
}

# A log line that names each value and a short hash of its stored bytes, not the value: the log goes into a public
# card.
function Format-KeyState($State) {
    $parts = foreach ($name in $State.Keys) { "$name=$(Get-Hash $State[$name])" }
    "$(@($State.Keys).Count) values: $($parts -join ', ')"
}

function Get-PathState {
    $type = [uint32]0
    $bytes = [WakeshotE2e.Reg]::Read($EnvSub, 'Path', [ref]$type)
    [pscustomobject]@{ Bytes = $bytes; Type = $type; Raw = [WakeshotE2e.Reg]::Raw($EnvSub, 'Path') }
}

function Get-PathEntries {
    $path = Get-RegValue $EnvSub 'Path'
    if ($null -eq $path) { return @() }
    @($path -split ';' | Where-Object { $_ } | ForEach-Object { [Environment]::ExpandEnvironmentVariables($_).TrimEnd('\') })
}

function Get-CurrentInPath {
    $want = $Current.TrimEnd('\')
    @(Get-PathEntries | Where-Object { [string]::Equals($_, $want, [StringComparison]::OrdinalIgnoreCase) }).Count
}

function Test-SamePath([string] $A, [string] $B) {
    [string]::Equals($A.TrimEnd('\'), $B.TrimEnd('\'), [StringComparison]::OrdinalIgnoreCase)
}

function Invoke-Powercfg([string[]] $Arguments) {
    $out = & powercfg @Arguments 2>&1 | Out-String
    if ($LASTEXITCODE -ne 0) { throw "powercfg $($Arguments -join ' ') exited with $LASTEXITCODE`: $($out.Trim())" }
    $out
}

function Get-ActivePlan {
    $out = Invoke-Powercfg @('/getactivescheme')
    if ($out -notmatch "GUID:\s*($GuidPattern)") { throw "powercfg /getactivescheme gave no plan: $out" }
    $Matches[1].ToLowerInvariant()
}

function Get-Plans {
    $out = Invoke-Powercfg @('/list')
    @(foreach ($line in $out -split "`r?`n") {
            if ($line -match "GUID:\s*($GuidPattern)\s+\((.*)\)") {
                [pscustomobject]@{ Id = $Matches[1].ToLowerInvariant(); Name = $Matches[2] }
            }
        })
}

# /qh, not /q: this PC hides the lid action and the energy saver threshold from /q.
function Get-PlanValue([string] $Plan, [string] $Sub, [string] $Setting) {
    $out = Invoke-Powercfg @('/qh', $Plan, $Sub, $Setting)
    $ac = [regex]::Match($out, 'Current AC Power Setting Index:\s*0x([0-9a-fA-F]+)')
    $dc = [regex]::Match($out, 'Current DC Power Setting Index:\s*0x([0-9a-fA-F]+)')
    if (-not ($ac.Success -and $dc.Success)) { throw "powercfg /qh $Plan $Sub $Setting gave no values" }
    [pscustomobject]@{ Ac = [Convert]::ToUInt32($ac.Groups[1].Value, 16); Dc = [Convert]::ToUInt32($dc.Groups[1].Value, 16) }
}

# The plan's values Wakeshot reads or writes, and all of its settings as text, so any change shows.
function Get-PlanState([string] $Plan) {
    $lid = Get-PlanValue $Plan 'SUB_BUTTONS' 'LIDACTION'
    $sleep = Get-PlanValue $Plan 'SUB_SLEEP' 'STANDBYIDLE'
    $saver = Get-PlanValue $Plan 'SUB_ENERGYSAVER' 'ESBATTTHRESHOLD'
    $all = Invoke-Powercfg @('/qh', $Plan)
    [ordered]@{
        LidAc = $lid.Ac; LidDc = $lid.Dc; SleepAc = $sleep.Ac; SleepDc = $sleep.Dc; SaverDc = $saver.Dc
        All = $all; AllHash = Get-Hash $all
    }
}

function Format-PlanState($State) {
    "lid AC $($State.LidAc) DC $($State.LidDc), sleep AC $($State.SleepAc) DC $($State.SleepDc), energy saver DC $($State.SaverDc), all settings $($State.AllHash)"
}

function Compare-PlanState($Want, $Got) {
    $diff = @(foreach ($k in 'LidAc', 'LidDc', 'SleepAc', 'SleepDc', 'SaverDc') {
            if ($Want[$k] -ne $Got[$k]) { "$k $($Want[$k]) -> $($Got[$k])" }
        })
    if ($Want.AllHash -ne $Got.AllHash) {
        $lines = @(Compare-Object ($Want.All -split "`r?`n") ($Got.All -split "`r?`n") | Select-Object -First 10 |
                ForEach-Object { "$($_.SideIndicator) $($_.InputObject.Trim())" })
        $diff += "all settings $($Want.AllHash) -> $($Got.AllHash): $($lines -join ' | ')"
    }
    $diff
}

function Test-RealPlan([string] $Check) {
    $active = Get-ActivePlan
    Test-Check "$Check`: active plan is still $($Before.Active)" ($active -eq $Before.Active) "active is $active"
    $diff = @(Compare-PlanState $Before.Plan (Get-PlanState $Before.Active))
    Test-Check "$Check`: real plan's values unchanged" ($diff.Count -eq 0) ($diff -join '; ')
}

# Runs an exe and waits; a run past the timeout is killed by its PID and reported as such. -Quiet keeps a console
# exe's output out of the report; it is only for exes that start no child, since a child that inherits the pipe would
# keep the read open.
function Invoke-Exe([string] $Exe, [string[]] $Arguments, [switch] $Quiet) {
    $info = [Diagnostics.ProcessStartInfo]::new($Exe)
    foreach ($a in $Arguments) { $info.ArgumentList.Add($a) }
    $info.UseShellExecute = $false
    $info.RedirectStandardOutput = [bool]$Quiet
    $watch = [Diagnostics.Stopwatch]::StartNew()
    $process = [Diagnostics.Process]::Start($info)
    $script:Started.Add([pscustomobject]@{ Pid = $process.Id; Start = $process.StartTime; What = (Split-Path $Exe -Leaf) })
    try {
        $output = if ($Quiet) { $process.StandardOutput.ReadToEndAsync() }
        if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
            $process.Kill($true)
            $script:LastSeconds = $watch.Elapsed.TotalSeconds
            return "timed out after $TimeoutSeconds s"
        }
        $script:LastSeconds = $watch.Elapsed.TotalSeconds
        if ($output) { $null = $output.Wait(5000) }
        $process.ExitCode
    } finally {
        $process.Dispose()
    }
}

# The journal's owner: a process this script starts and that has ended, so the hooks see it Gone.
function New-EndedOwner {
    $info = [Diagnostics.ProcessStartInfo]::new((Join-Path ([Environment]::SystemDirectory) 'PING.EXE'))
    foreach ($a in '-n', '2', '127.0.0.1') { $info.ArgumentList.Add($a) }
    $info.UseShellExecute = $false
    $info.RedirectStandardOutput = $true
    $info.CreateNoWindow = $true
    $process = [Diagnostics.Process]::Start($info)
    try {
        $script:Started.Add([pscustomobject]@{ Pid = $process.Id; Start = $process.StartTime; What = 'PING.EXE (journal owner)' })
        $owner = [pscustomobject]@{
            Pid = $process.Id; Start = "$($process.StartTime.ToFileTimeUtc())"; Session = $process.SessionId
        }
        $null = $process.StandardOutput.ReadToEnd()
        if (-not $process.WaitForExit(10000)) {
            $process.Kill($true)
            throw "the journal owner (pid $($owner.Pid)) did not end"
        }
    } finally {
        $process.Dispose()
    }
    $owner
}

# The hold an apply leaves: the copy's DC lid action is ours (Do nothing), the journal says so, its owner is gone.
function Set-Hold([string] $Check) {
    $null = Invoke-Powercfg @('/setdcvalueindex', $script:Copy, 'SUB_BUTTONS', 'LIDACTION', '0')
    $live = (Get-PlanValue $script:Copy 'SUB_BUTTONS' 'LIDACTION').Dc
    Test-Check "$Check`: the copy's DC lid action is Do nothing (0)" ($live -eq 0) "reads $live"
    $owner = New-EndedOwner
    $file = [ordered]@{
        version = 1
        owner = [ordered]@{ pid = $owner.Pid; procStart = $owner.Start; session = $owner.Session }
        entries = @([ordered]@{ plan = $script:Copy; key = 'lidDc'; original = $script:CopyLidDc; written = 0; restored = $false })
    }
    $json = $file | ConvertTo-Json -Depth 5 -Compress
    New-Item -ItemType Directory -Force -Path $DataDir | Out-Null
    [IO.File]::WriteAllText($Journal, $json, [Text.UTF8Encoding]::new($false))
    Say "INFO  $Check`: journal.json $json (owner pid $($owner.Pid) ended)"
    $live
}

function Test-HoldBack([string] $Check) {
    $live = (Get-PlanValue $script:Copy 'SUB_BUTTONS' 'LIDACTION').Dc
    Test-Check "$Check`: the copy's DC lid action is back to $($script:CopyLidDc)" ($live -eq $script:CopyLidDc) "reads $live"
    Test-Check "$Check`: journal.json gone" (-not (Test-Path -LiteralPath $Journal)) "$Journal still there"
}

# A process started from the install folder: hooks end before Setup and Update.exe exit, so after them any such
# process is a started tray, which would hold this PC awake for real.
function Get-InstalledProcesses {
    @(Get-CimInstance Win32_Process | Where-Object {
            $_.ExecutablePath -and $_.ExecutablePath.StartsWith("$Root\", [StringComparison]::OrdinalIgnoreCase)
        })
}

function Test-NoneStarted([string] $Check) {
    Start-Sleep -Seconds 2
    $found = @(Get-InstalledProcesses)
    foreach ($p in $found) {
        Say "INFO  $Check`: stopping pid $($p.ProcessId) $($p.ExecutablePath) $($p.CommandLine)"
        Stop-Process -Id $p.ProcessId -Force -ErrorAction SilentlyContinue
    }
    $names = ($found | ForEach-Object { "pid $($_.ProcessId) $($_.Name)" }) -join ', '
    Test-Check "$Check`: no Wakeshot process runs" ($found.Count -eq 0) "found and stopped $names"
}

function Wait-Gone([string] $Path, [int] $Seconds) {
    # Update.exe cannot delete itself, so the install folder goes about 3 s after it exits (research.md, "Velopack
    # checks for Wakeshot", Uninstall).
    $until = (Get-Date).AddSeconds($Seconds)
    while ((Test-Path -LiteralPath $Path) -and (Get-Date) -lt $until) { Start-Sleep -Milliseconds 250 }
    -not (Test-Path -LiteralPath $Path)
}

function Get-FileLength([string] $Path) {
    if (Test-Path -LiteralPath $Path -PathType Leaf) { (Get-Item -LiteralPath $Path).Length } else { 0 }
}

# Logs keep growing across steps, so each step reads only what it added. Delete sharing, as every Wakeshot process
# opens the log.
function Read-NewText([string] $Path, [long] $From) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { return '' }
    $stream = [IO.File]::Open($Path, 'Open', 'Read', 'ReadWrite, Delete')
    try {
        if ($From -gt $stream.Length) { $From = 0 }
        $stream.Position = $From
        [IO.StreamReader]::new($stream).ReadToEnd()
    } finally {
        $stream.Dispose()
    }
}

function Show-Lines([string] $Title, [string] $Text) {
    $lines = @($Text -split "`r?`n" | Where-Object { $_ })
    Say "---- $Title ($($lines.Count) lines)"
    $lines | ForEach-Object { Say "  $_" }
}

# Checks the hook log lines a step added, in order: each pattern must match a line after the one before matched.
function Test-HookLines([string] $Check, [string] $Text, [string[]] $Patterns) {
    $lines = @($Text -split "`r?`n" | Where-Object { $_ })
    $at = 0
    foreach ($pattern in $Patterns) {
        $hit = -1
        for ($i = $at; $i -lt $lines.Count; $i++) { if ($lines[$i] -match $pattern) { $hit = $i; break } }
        Test-Check "$Check`: hook log line /$pattern/" ($hit -ge 0) 'not found in the lines this step added'
        if ($hit -ge 0) { $at = $hit + 1 }
    }
}

function Hook-Line([string] $Text) { 'hook\s+INFO\s+hook: ' + [regex]::Escape($Text) }

function Get-QuietUninstall {
    $quiet = Get-RegValue $UninstallSub 'QuietUninstallString'
    if ($quiet -match '^"([^"]+)"\s*(.*)$') {
        return [pscustomobject]@{ Exe = $Matches[1]; Args = @($Matches[2] -split '\s+' | Where-Object { $_ }); Text = $quiet }
    }
    [pscustomobject]@{ Exe = $UpdateExe; Args = @('--uninstall', '--silent'); Text = $quiet }
}

function Get-Traces {
    $traces = @()
    if (Test-Path -LiteralPath $Root) { $traces += "install folder $Root" }
    if (Test-RegKey $UninstallSub) { $traces += "uninstall key HKCU\$UninstallSub" }
    if ($null -ne (Get-RegValue $RunSub $RunName)) { $traces += "Run value $RunName" }
    if ($null -ne (Get-RegValue $RunOnceSub $RunOnceName)) { $traces += "RunOnce value $RunOnceName" }
    if ((Get-CurrentInPath) -gt 0) { $traces += "user Path entry $Current" }
    if (Test-Path -LiteralPath $Shortcut) { $traces += "shortcut $Shortcut" }
    if (Test-Path -LiteralPath $VeloTemp) { $traces += "temp folder $VeloTemp" }
    # Setup over an install renames the old folder to HaikAsatryan.<random> for its rollback (research.md, point 1).
    $traces += @(Get-ChildItem -LiteralPath $LocalAppData -Directory -Filter 'HaikAsatryan.*' -Force |
            Where-Object FullName -ne $Root | ForEach-Object { "folder $($_.FullName)" })
    $traces
}

# ---- Refusals: nothing has changed yet.

$groups = (whoami /groups) -join "`n"
$level = if ($groups -match 'Mandatory Label\\(\w+) Mandatory Level') { $Matches[1] } else { 'unknown' }
$principal = [Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
$elevated = $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
Say "e2e: user $env:USERNAME, integrity $level, elevated $elevated, pid $PID, $((Get-Date).ToString('yyyy-MM-dd HH:mm:ss'))"
if (($elevated -or $level -ne 'Medium') -and -not $AllowElevated) {
    Say 'e2e: refused: run it unelevated (Medium), as users run Wakeshot, or pass -AllowElevated on a CI runner.'
    exit 2
}

foreach ($exe in 'wakeshot-tray.exe', 'wakeshot.exe') {
    if (-not (Test-Path -LiteralPath (Join-Path $ExeDir $exe) -PathType Leaf)) {
        Say "e2e: refused: $(Join-Path $ExeDir $exe) not found"
        exit 2
    }
}
if ((Test-Path -LiteralPath $WorkDir) -and (Get-ChildItem -LiteralPath $WorkDir -Force | Select-Object -First 1)) {
    Say "e2e: refused: $WorkDir is not empty; give a new or empty folder"
    exit 2
}
$traces = @(Get-Traces)
if ($traces) {
    Say "e2e: refused, Wakeshot is or was here: $($traces -join '; '). Uninstall it and remove these first."
    exit 2
}
if (Test-Path -LiteralPath $Journal) {
    Say "e2e: refused: $Journal exists, so Wakeshot may own power settings now. Run ``wakeshot restore`` first."
    exit 2
}
$running = @(Get-Process -ErrorAction SilentlyContinue | Where-Object { $_.ProcessName -like 'wakeshot*' })
foreach ($p in $running) { Say "INFO  a Wakeshot process from elsewhere runs: pid $($p.Id) $($p.Path)" }

$active = Get-ActivePlan
$leftovers = @(Get-Plans | Where-Object Name -ceq $CopyName)
if ($leftovers | Where-Object Id -eq $active) {
    Say "e2e: refused: the active plan is a '$CopyName' copy from a cut run. Switch to your own plan first."
    exit 2
}

New-Item -ItemType Directory -Force -Path $WorkDir | Out-Null
$WorkDir = (Resolve-Path -LiteralPath $WorkDir).Path
$ExeDir = (Resolve-Path -LiteralPath $ExeDir).Path
$setupLog = Join-Path $VelopackLogs 'velopack.log'
$updateLog = Join-Path $VelopackLogs "velopack_$PackId.log"
$setupLogFrom = Get-FileLength $setupLog
$updateLogFrom = Get-FileLength $updateLog

# ---- Step 1: BEFORE.

Step 'step 1: BEFORE'
$Before = [ordered]@{
    Run = Get-KeyState $RunSub
    RunOnce = Get-KeyState $RunOnceSub
    Path = Get-PathState
    Active = $active
    Plans = @(Get-Plans | Where-Object Name -cne $CopyName | ForEach-Object { "$($_.Id) $($_.Name)" })
    Plan = Get-PlanState $active
}
Say "INFO  Run: $(Format-KeyState $Before.Run)"
Say "INFO  RunOnce: $(Format-KeyState $Before.RunOnce)"
$pathLength = if ($null -eq $Before.Path.Bytes) { 'missing' } else { "$($Before.Path.Bytes.Length) bytes" }
Say "INFO  user Path: type $($Before.Path.Type), $pathLength, $(Get-Hash $Before.Path.Raw)"
Say "INFO  plans: $($Before.Plans -join '; '); active $active"
Say "INFO  active plan: $(Format-PlanState $Before.Plan)"
# Kept in the work folder only: the values themselves, for a restore by hand if this script is killed.
$Before | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $WorkDir 'before.json') -Encoding utf8NoBOM
Say "INFO  BEFORE saved to $(Join-Path $WorkDir 'before.json')"
Test-Check 'step 1: no HaikAsatryan.Wakeshot install' ($traces.Count -eq 0) ($traces -join '; ')

$movedData = $null
if (Test-Path -LiteralPath $DataDir) {
    $movedData = Join-Path $WorkDir 'data-before\Wakeshot'
    New-Item -ItemType Directory -Force -Path (Split-Path $movedData) | Out-Null
    Move-Item -LiteralPath $DataDir -Destination $movedData
    Say "INFO  step 1: $DataDir moved to $movedData"
} else {
    Say "INFO  step 1: no $DataDir to move"
}

$script:Copy = $null
$script:CopyLidDc = $null
$installedOnce = $false
try {
    # ---- Step 2: the plan copy.

    Step "step 2: plan copy '$CopyName'"
    foreach ($old in $leftovers) {
        $null = Invoke-Powercfg @('/delete', $old.Id)
        Say "INFO  step 2: deleted a leftover '$CopyName' copy $($old.Id) from a cut run"
    }
    $out = Invoke-Powercfg @('/duplicatescheme', $active)
    if ($out -notmatch "GUID:\s*($GuidPattern)") { throw "powercfg /duplicatescheme gave no plan: $out" }
    $script:Copy = $Matches[1].ToLowerInvariant()
    $null = Invoke-Powercfg @('/changename', $script:Copy, $CopyName)
    $script:CopyLidDc = (Get-PlanValue $script:Copy 'SUB_BUTTONS' 'LIDACTION').Dc
    Say "INFO  step 2: copy $($script:Copy), its DC lid action $($script:CopyLidDc)"
    Test-Check 'step 2: copy listed under its name' ([bool](Get-Plans | Where-Object { $_.Id -eq $script:Copy -and $_.Name -ceq $CopyName })) 'not in powercfg /list'
    Test-Check "step 2: the copy's DC lid action is not Do nothing already" ($script:CopyLidDc -ne 0) 'it is 0, so a restore could not be told apart'
    Test-RealPlan 'step 2'

    # ---- Step 3: packs.

    Step 'step 3: pack 0.1.0 and 0.1.1'
    $pack = Join-Path $PSScriptRoot 'pack.ps1'
    $packDir = @{ $OldVersion = Join-Path $WorkDir "packs\$OldVersion"; $NewVersion = Join-Path $WorkDir "packs\$NewVersion" }
    foreach ($v in $OldVersion, $NewVersion) {
        $packArgs = @{ Version = $v; Cpu = 'x64'; ExeDir = $ExeDir; OutDir = $packDir[$v]; Vpk = $Vpk }
        if (Test-Path -LiteralPath $Icon -PathType Leaf) { $packArgs.Icon = $Icon }
        if ($v -eq $NewVersion) { $packArgs.PreviousDir = $packDir[$OldVersion] }
        $packText = & $pack @packArgs *>&1 | Out-String
        $packCode = $LASTEXITCODE
        Set-Content -LiteralPath (Join-Path $WorkDir "pack-$v.log") -Value $packText -Encoding utf8NoBOM
        @($packText -split "`r?`n" | Where-Object { $_ -match '^pack: ' }) | ForEach-Object { Say "  $_" }
        if ($packCode -ne 0) { throw "pack.ps1 for $v exited with $packCode (pack-$v.log)" }
    }
    $Setup = Join-Path $packDir[$OldVersion] "$PackId-win-x64-Setup.exe"
    $NewFull = Join-Path $packDir[$NewVersion] "$PackId-$NewVersion-win-x64-full.nupkg"
    foreach ($f in @(Get-ChildItem -LiteralPath $packDir[$OldVersion], $packDir[$NewVersion] -File)) {
        Say ('INFO  step 3: {0,12:N0} bytes  {1}' -f $f.Length, (Join-Path (Split-Path $f.DirectoryName -Leaf) $f.Name))
    }
    $manifest = [Text.Encoding]::ASCII.GetString([IO.File]::ReadAllBytes($Setup))
    $asked = if ($manifest -match 'requestedExecutionLevel\s+level=.([A-Za-z]+)') { $Matches[1] } else { 'none found' }
    Say "INFO  step 3: Setup.exe manifest asks for execution level $asked"

    # ---- Step 4: silent install.

    Step "step 4: Setup.exe --silent of $OldVersion"
    $hookFrom = 0
    $code = Invoke-Exe $Setup @('--silent')
    $installedOnce = $true
    $script:Times['install'] = $script:LastSeconds
    Test-Check 'step 4: silent install exits 0' ($code -eq 0) "exit $code"
    Say ('INFO  step 4: install took {0:N1} s' -f $script:LastSeconds)
    Test-NoneStarted 'step 4'
    foreach ($file in 'Update.exe', 'Wakeshot.exe', 'current\wakeshot-tray.exe', 'current\wakeshot.exe', 'current\sq.version') {
        $path = Join-Path $Root $file
        Test-Check "step 4: file $file" (Test-Path -LiteralPath $path -PathType Leaf) "$path not found"
    }
    if (Test-Path -LiteralPath $Shortcut) {
        $target = (New-Object -ComObject WScript.Shell).CreateShortcut($Shortcut).TargetPath
        Test-Check 'step 4: Start menu shortcut' (Test-SamePath $target $TrayExe) "$Shortcut points at '$target', want $TrayExe"
    } else {
        Fail 'step 4: Start menu shortcut' "$Shortcut not found"
    }
    if (Test-RegKey $UninstallSub) {
        Test-Check 'step 4: uninstall key DisplayName' ((Get-RegValue $UninstallSub 'DisplayName') -ceq 'Wakeshot') "'$(Get-RegValue $UninstallSub 'DisplayName')'"
        Test-Check 'step 4: uninstall key Publisher' ((Get-RegValue $UninstallSub 'Publisher') -ceq 'Haik Asatryan') "'$(Get-RegValue $UninstallSub 'Publisher')'"
        Test-Check "step 4: uninstall key DisplayVersion is $OldVersion" ((Get-RegValue $UninstallSub 'DisplayVersion') -ceq $OldVersion) "'$(Get-RegValue $UninstallSub 'DisplayVersion')'"
        Test-Check 'step 4: uninstall key QuietUninstallString' ([bool](Get-RegValue $UninstallSub 'QuietUninstallString')) 'missing'
    } else {
        Fail 'step 4: uninstall key' "HKCU\$UninstallSub not found"
    }
    $run = Get-RegValue $RunSub $RunName
    Test-Check "step 4: Run value $RunName is the quoted current\wakeshot-tray.exe" ($run -ceq $RunCommand) "'$run', want '$RunCommand'"
    Test-Check 'step 4: user Path has the current folder once' ((Get-CurrentInPath) -eq 1) "$(Get-CurrentInPath) entries"
    $hookText = Read-NewText $HookLog $hookFrom
    $hookFrom = Get-FileLength $HookLog
    Show-Lines "step 4: $HookLog, this step" $hookText
    Test-HookLines 'step 4' $hookText @(
        (Hook-Line "install hook for version $OldVersion started"),
        (Hook-Line "Run value $RunName set to $RunCommand"),
        # WARN when telling running programs failed, which this check does not judge.
        'hook\s+(INFO|WARN)\s+hook: CLI folder added to the user Path',
        (Hook-Line 'restore: no journal, nothing to restore'),
        (Hook-Line 'install hook done after'))
    # The winget pipeline runs installed exes with no arguments and flags a non-zero exit (research/distribution.md,
    # section 2). The tray is not run: it would start Wakeshot for real.
    $code = Invoke-Exe (Join-Path $Current 'wakeshot.exe') @() -Quiet
    Test-Check 'step 4: wakeshot.exe with no arguments exits 0' ($code -eq 0) "exit $code"
    Test-RealPlan 'step 4'

    # ---- Step 5: Setup over the install restores.

    Step "step 5: hold, then Setup.exe --silent of $OldVersion over the install"
    $null = Set-Hold 'step 5'
    $code = Invoke-Exe $Setup @('--silent')
    $script:Times['setup over install'] = $script:LastSeconds
    Test-Check 'step 5: Setup over the install exits 0' ($code -eq 0) "exit $code"
    Say ('INFO  step 5: Setup over the install took {0:N1} s' -f $script:LastSeconds)
    Test-NoneStarted 'step 5'
    Test-HoldBack 'step 5'
    Test-RealPlan 'step 5'
    $hookText = Read-NewText $HookLog $hookFrom
    $hookFrom = Get-FileLength $HookLog
    Show-Lines "step 5: $HookLog, this step" $hookText
    Test-HookLines 'step 5' $hookText @(
        (Hook-Line "install hook for version $OldVersion started"),
        (Hook-Line "restore: the journal's values are back"),
        (Hook-Line 'install hook done after'))
    Test-Check "step 5: Run value $RunName unchanged" ((Get-RegValue $RunSub $RunName) -ceq $RunCommand) "'$(Get-RegValue $RunSub $RunName)'"
    Test-Check 'step 5: user Path has the current folder once' ((Get-CurrentInPath) -eq 1) "$(Get-CurrentInPath) entries"

    # ---- Step 6: an update restores.

    Step "step 6: hold, then Update.exe apply of $NewVersion"
    $null = Set-Hold 'step 6'
    $staged = Join-Path $Packages (Split-Path $NewFull -Leaf)
    New-Item -ItemType Directory -Force -Path $Packages | Out-Null
    Copy-Item -LiteralPath $NewFull -Destination $staged
    $currentBorn = (Get-Item -LiteralPath $Current).CreationTimeUtc
    $code = Invoke-Exe $UpdateExe @('apply', '--silent', '--norestart', '--package', $staged)
    $script:Times['update'] = $script:LastSeconds
    Test-Check 'step 6: Update.exe apply exits 0' ($code -eq 0) "exit $code"
    Say ('INFO  step 6: apply took {0:N1} s' -f $script:LastSeconds)
    Test-NoneStarted 'step 6'
    Test-HoldBack 'step 6'
    Test-RealPlan 'step 6'
    $hookText = Read-NewText $HookLog $hookFrom
    $hookFrom = Get-FileLength $HookLog
    Show-Lines "step 6: $HookLog, this step" $hookText
    Test-HookLines 'step 6' $hookText @(
        (Hook-Line "obsolete hook for version $OldVersion"),
        (Hook-Line "updated hook for version $NewVersion started"),
        (Hook-Line "restore: the journal's values are back"),
        (Hook-Line 'updated hook done after'))
    $sq = Join-Path $Current 'sq.version'
    $sqText = if (Test-Path -LiteralPath $sq) { Get-Content -LiteralPath $sq -Raw } else { '' }
    Test-Check "step 6: current\sq.version is $NewVersion" ($sqText -match "<version>$([regex]::Escape($NewVersion))</version>") 'no such version element'
    Test-Check "step 6: uninstall key DisplayVersion is $NewVersion" ((Get-RegValue $UninstallSub 'DisplayVersion') -ceq $NewVersion) "'$(Get-RegValue $UninstallSub 'DisplayVersion')'"
    Test-Check 'step 6: current keeps its path' (Test-Path -LiteralPath $TrayExe -PathType Leaf) "$TrayExe not found"
    $newBorn = if (Test-Path -LiteralPath $Current) { (Get-Item -LiteralPath $Current).CreationTimeUtc } else { $null }
    Say "INFO  step 6: current created $($currentBorn.ToString('o')) before, $(if ($newBorn) { $newBorn.ToString('o') } else { 'none' }) after"
    Test-Check "step 6: Run value $RunName unchanged" ((Get-RegValue $RunSub $RunName) -ceq $RunCommand) "'$(Get-RegValue $RunSub $RunName)'"
    Test-Check 'step 6: user Path has the current folder once' ((Get-CurrentInPath) -eq 1) "$(Get-CurrentInPath) entries"

    # ---- Step 7: an uninstall during a hold restores and removes everything.

    Step 'step 7: hold and RunOnce, then Update.exe --uninstall --silent'
    $null = Set-Hold 'step 7'
    $runOnceKey = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey($RunOnceSub)
    try { $runOnceKey.SetValue($RunOnceName, $RunOnceCommand, 'String') } finally { $runOnceKey.Dispose() }
    Test-Check "step 7: RunOnce value $RunOnceName set" ((Get-RegValue $RunOnceSub $RunOnceName) -ceq $RunOnceCommand) "'$(Get-RegValue $RunOnceSub $RunOnceName)'"
    $hookText = Read-NewText $HookLog 0
    Set-Content -LiteralPath (Join-Path $WorkDir 'wakeshot-before-uninstall.log') -Value $hookText -Encoding utf8NoBOM
    $quiet = Get-QuietUninstall
    $wantQuiet = "`"$UpdateExe`" --uninstall --silent"
    Test-Check 'step 7: QuietUninstallString is Update.exe --uninstall --silent' ($quiet.Text -ceq $wantQuiet) "'$($quiet.Text)'"
    $code = Invoke-Exe $quiet.Exe $quiet.Args
    $script:Times['uninstall'] = $script:LastSeconds
    Test-Check 'step 7: silent uninstall exits 0' ($code -eq 0) "exit $code"
    $watch = [Diagnostics.Stopwatch]::StartNew()
    $gone = Wait-Gone $Root 30
    $script:Times['install folder gone after uninstall exit'] = $watch.Elapsed.TotalSeconds
    Say ('INFO  step 7: uninstall took {0:N1} s; the install folder went {1:N1} s after its exit' -f $script:Times['uninstall'], $watch.Elapsed.TotalSeconds)
    Test-NoneStarted 'step 7'
    $live = (Get-PlanValue $script:Copy 'SUB_BUTTONS' 'LIDACTION').Dc
    Test-Check "step 7: the copy's DC lid action is back to $($script:CopyLidDc)" ($live -eq $script:CopyLidDc) "reads $live"
    Test-RealPlan 'step 7'
    Test-Check "step 7: Run value $RunName gone" ($null -eq (Get-RegValue $RunSub $RunName)) 'still there'
    Test-Check "step 7: RunOnce value $RunOnceName gone" ($null -eq (Get-RegValue $RunOnceSub $RunOnceName)) 'still there'
    Test-Check 'step 7: user Path entry gone' ((Get-CurrentInPath) -eq 0) "$Current still in Path"
    Test-Check 'step 7: user Path has its old type and bytes' ((Get-PathState).Raw -ceq $Before.Path.Raw) 'differs from BEFORE'
    # Question 2 of the ship track: the data folder goes after a good restore.
    Test-Check 'step 7: data folder gone' (Wait-Gone $DataDir 5) "$DataDir still there"
    Test-Check 'step 7: install folder gone' $gone "$Root still there"
    Test-Check 'step 7: uninstall key gone' (-not (Test-RegKey $UninstallSub)) "HKCU\$UninstallSub still there"
    Test-Check 'step 7: Start menu shortcut gone' (-not (Test-Path -LiteralPath $Shortcut)) "$Shortcut still there"
    Test-Check "step 7: $VeloTemp gone" (Wait-Gone $VeloTemp 5) 'still there'
} catch {
    Fail 'run' "stopped at line $($_.InvocationInfo.ScriptLineNumber): $($_.Exception.Message)"
} finally {
    # ---- Step 8: AFTER. Each part runs even when one before it fails.

    Step 'step 8: AFTER'
    $tidy = @(
        {
            if ((Test-RegKey $UninstallSub) -or (Test-Path -LiteralPath $Root)) {
                $quiet = Get-QuietUninstall
                if (Test-Path -LiteralPath $quiet.Exe -PathType Leaf) {
                    $code = Invoke-Exe $quiet.Exe $quiet.Args
                    Fail 'step 8: Wakeshot was still installed' "uninstalled now, exit $code"
                } else {
                    Fail 'step 8: Wakeshot was still installed' "$($quiet.Exe) not found"
                }
                $null = Wait-Gone $Root 30
            }
        },
        {
            foreach ($p in Get-InstalledProcesses) {
                Stop-Process -Id $p.ProcessId -Force -ErrorAction SilentlyContinue
                Fail 'step 8: a process from the install folder ran' "stopped pid $($p.ProcessId) $($p.ExecutablePath)"
            }
        },
        {
            # Refused at the start when either was there, so any value now is this run's.
            foreach ($v in @(@($RunSub, $RunName), @($RunOnceSub, $RunOnceName))) {
                if ($null -ne (Get-RegValue $v[0] $v[1])) {
                    [WakeshotE2e.Reg]::Delete($v[0], $v[1])
                    Fail "step 8: $($v[1]) was still there" 'removed now'
                }
            }
        },
        {
            if ((Get-PathState).Raw -cne $Before.Path.Raw) {
                if ($null -eq $Before.Path.Bytes) {
                    [WakeshotE2e.Reg]::Delete($EnvSub, 'Path')
                } else {
                    [WakeshotE2e.Reg]::Write($EnvSub, 'Path', $Before.Path.Type, $Before.Path.Bytes)
                }
                $null = [WakeshotE2e.Reg]::Broadcast()
                Fail 'step 8: user Path differed from BEFORE' 'BEFORE bytes written back'
            }
        },
        {
            $now = Get-ActivePlan
            if ($now -ne $Before.Active) {
                $null = Invoke-Powercfg @('/setactive', $Before.Active)
                Fail 'step 8: active plan differed from BEFORE' "was $now, BEFORE's plan made active again"
            }
        },
        {
            $now = Get-PlanState $Before.Active
            $writes = @()
            foreach ($k in @(@('LidAc', 'SUB_BUTTONS', 'LIDACTION', '/setacvalueindex'), @('LidDc', 'SUB_BUTTONS', 'LIDACTION', '/setdcvalueindex'),
                    @('SleepAc', 'SUB_SLEEP', 'STANDBYIDLE', '/setacvalueindex'), @('SleepDc', 'SUB_SLEEP', 'STANDBYIDLE', '/setdcvalueindex'))) {
                if ($now[$k[0]] -ne $Before.Plan[$k[0]]) {
                    $null = Invoke-Powercfg @($k[3], $Before.Active, $k[1], $k[2], "$($Before.Plan[$k[0]])")
                    $writes += "$($k[0]) $($now[$k[0]]) -> $($Before.Plan[$k[0]])"
                }
            }
            if ($writes) {
                # A stored value is in force only after the plan is set active again.
                $null = Invoke-Powercfg @('/setactive', $Before.Active)
                Fail "step 8: the real plan's values differed from BEFORE" "written back: $($writes -join '; ')"
            }
        },
        {
            foreach ($p in @(Get-Plans | Where-Object { $_.Id -eq $script:Copy -or $_.Name -ceq $CopyName })) {
                $null = Invoke-Powercfg @('/delete', $p.Id)
                Say "INFO  step 8: deleted plan copy $($p.Id)"
            }
        },
        {
            if (Test-Path -LiteralPath $DataDir) {
                # What a failed restore keeps (journal.json and logs), kept here for the record.
                $left = Join-Path $WorkDir 'data-left'
                Move-Item -LiteralPath $DataDir -Destination $left
                Say "INFO  step 8: $DataDir was there and is moved to $left"
            }
            if ($movedData) {
                Move-Item -LiteralPath $movedData -Destination $DataDir
                Say "INFO  step 8: $movedData moved back to $DataDir"
            }
        },
        {
            if ($installedOnce -and (Test-Path -LiteralPath $VeloTemp)) {
                Remove-Item -LiteralPath $VeloTemp -Recurse -Force
                Fail 'step 8: the uninstall left the temp folder' "$VeloTemp deleted now"
            }
        }
    )
    foreach ($part in $tidy) {
        try { & $part } catch { Fail 'step 8: tidy part' "line $($_.InvocationInfo.ScriptLineNumber): $($_.Exception.Message)" }
    }

    try {
        $diff = @(Compare-KeyState $Before.Run (Get-KeyState $RunSub))
        Test-Check 'step 8: Run key equals BEFORE' ($diff.Count -eq 0) ($diff -join '; ')
        $diff = @(Compare-KeyState $Before.RunOnce (Get-KeyState $RunOnceSub))
        Test-Check 'step 8: RunOnce key equals BEFORE' ($diff.Count -eq 0) ($diff -join '; ')
        $pathNow = Get-PathState
        Test-Check 'step 8: user Path type and bytes equal BEFORE' ($pathNow.Raw -ceq $Before.Path.Raw) "BEFORE $(Get-Hash $Before.Path.Raw), now $(Get-Hash $pathNow.Raw)"
        $now = Get-ActivePlan
        Test-Check "step 8: active plan equals BEFORE ($($Before.Active))" ($now -eq $Before.Active) "now $now"
        $planNow = Get-PlanState $Before.Active
        $diff = @(Compare-PlanState $Before.Plan $planNow)
        Say "INFO  step 8: active plan now: $(Format-PlanState $planNow)"
        Test-Check "step 8: real plan's values equal BEFORE" ($diff.Count -eq 0) ($diff -join '; ')
        $plans = @(Get-Plans | ForEach-Object { "$($_.Id) $($_.Name)" })
        Test-Check "step 8: no '$CopyName' plan left, plan list equals BEFORE" (($plans -join '; ') -ceq ($Before.Plans -join '; ')) "now: $($plans -join '; ')"
        $traces = @(Get-Traces)
        Test-Check 'step 8: no Wakeshot install trace' ($traces.Count -eq 0) ($traces -join '; ')
        Test-Check 'step 8: data folder as BEFORE' ((Test-Path -LiteralPath $DataDir) -eq [bool]$movedData) "exists: $(Test-Path -LiteralPath $DataDir)"
        $alive = @(foreach ($s in $script:Started) {
                $p = Get-Process -Id $s.Pid -ErrorAction SilentlyContinue
                # A reused pid of a process this user cannot open has an unreadable start, so it is not ours.
                $same = if ($p) { try { $p.StartTime -eq $s.Start } catch { $false } } else { $false }
                if ($same) { $s }
            })
        foreach ($s in $alive) { Stop-Process -Id $s.Pid -Force -ErrorAction SilentlyContinue }
        Test-Check "step 8: none of the $($script:Started.Count) processes this run started is left" ($alive.Count -eq 0) "stopped: $(($alive | ForEach-Object { "pid $($_.Pid) $($_.What)" }) -join ', ')"
        Say "INFO  step 8: started: $(($script:Started | ForEach-Object { "$($_.What) pid $($_.Pid)" }) -join ', ')"
    } catch {
        Fail 'step 8: AFTER check' "line $($_.InvocationInfo.ScriptLineNumber): $($_.Exception.Message)"
    }

    $times = ($script:Times.Keys | ForEach-Object { '{0} {1:N1} s' -f $_, $script:Times[$_] }) -join ', '
    Say "INFO  times: $times"
    Set-Content -LiteralPath (Join-Path $WorkDir 'velopack-setup.log') -Value (Read-NewText $setupLog $setupLogFrom) -Encoding utf8NoBOM
    Set-Content -LiteralPath (Join-Path $WorkDir 'velopack-update.log') -Value (Read-NewText $updateLog $updateLogFrom) -Encoding utf8NoBOM
    Say "INFO  Velopack's log lines of this run saved to $WorkDir (velopack-setup.log, velopack-update.log)"
}

if ($script:Failed) {
    Say "e2e: $script:Failed check(s) failed"
    exit 1
}
Say 'e2e: all checks passed'
exit 0
