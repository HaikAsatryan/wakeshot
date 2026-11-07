#Requires -Version 7

<#
.SYNOPSIS
Runs Wakeshot's update path on a real install on this PC: an update found in a local feed waits while Keep awake
holds and applies once it ends; Setup.exe over the install (what winget upgrade runs) and an uninstall, each during a
real hold, put the power values back and leave the PC as it was. One line per check; exit 0 when all pass, 1 when any
fails, 2 when it refuses to run.

.DESCRIPTION
The steps:

1. BEFORE: the Run and RunOnce keys, the user Path (type and bytes), the active plan (the user's own) and its lid,
   sleep and energy saver values, the plan list. A data folder %LocalAppData%\Wakeshot is moved into -WorkDir.
2. Packs 0.0.1, 0.0.2 and 0.0.3 of the debug exes in -ExeDir with pack.ps1, and a local feed folder that holds only
   0.0.1's release files. Debug exes only: a release exe never reads WAKESHOT_UPDATE_FEED, so it refuses them.
3. A copy of the user's plan named "Wakeshot int update", made active.
4. Setup.exe --silent of 0.0.1 and the install checks; then the installed tray, started at Medium through
   explorer.exe and a .cmd that sets WAKESHOT_UPDATE_FEED and an empty Claude folder (CLAUDE_CONFIG_DIR), so the agent
   sessions on this PC never hold it.
5. wakeshot keep-awake 30m: the copy is overridden. 0.0.2 goes into the feed; the first check, 5 minutes after the
   start, downloads it, and for 2 more minutes nothing applies it. keep-awake off: the copy is put back, then the tray
   quits, Velopack applies 0.0.2 and starts the new tray, which logs the restart and shows the update notice.
6. Keep awake on and wakeshot run -- ping -n 600 127.0.0.1, then Setup.exe --silent of 0.0.3 over the install, as
   winget upgrade runs it: the copy's values back, no journal, no RunOnce value, no tray left running. What happened
   to the wrapper and its ping is recorded; a ping that still runs is ended by its pid.
7. The tray started again and Keep awake on, then the uninstall key's QuietUninstallString: the copy's values back;
   Run, RunOnce and the Path entry gone; the data, install and temp folders, the uninstall key and the shortcut gone.
8. AFTER: the user's plan active again, the copy deleted, the data folder put back, every BEFORE value compared, no
   process this run started left.

The tray writes the active plan, so the user's plan is never active while a Wakeshot this script installed runs: the
copy is active from before the install, and the user's plan is made active again only once no process from the
install folder runs. A hold never names the user's plan, so no step can change it unless something writes the wrong
plan.

Whatever stops the run, the end uninstalls, ends every process from the install folder and the wrapper's ping,
removes Run and RunOnce values this run left, puts back the user Path bytes, makes the user's plan active again,
writes its values back if they differ from BEFORE, deletes the copy and moves the data folder back. A run that is
killed does none of this. Then, in this order: uninstall Wakeshot (Settings > Apps, or
"%LocalAppData%\HaikAsatryan.Wakeshot\Update.exe" --uninstall --silent); powercfg /setactive with the Active id in
-WorkDir\before.json; powercfg /delete with the id of the "Wakeshot int update" plan (powercfg /list); move
-WorkDir\data-before\Wakeshot back into %LocalAppData%.

Run it unelevated (Medium), as users run Wakeshot: an elevated run installs with a token no user has. -AllowElevated
is for a CI runner, where every process is elevated. It refuses while Wakeshot is installed or was not fully removed,
a journal exists, a Wakeshot process runs, or the active plan is a "Wakeshot ..." test copy. -PrepareOnly runs only
what leaves the PC as it is: those checks (reported, not enforced), the BEFORE read, and the packs and the feed in
-WorkDir; it may run elevated.

Where the terminal opens elevated (the test laptop's does), a .cmd opened with explorer.exe runs at Medium
(research.md, "Medium launches through Explorer and console stop events"); the third example is such a .cmd. Write
the exit code as `(echo %ERRORLEVEL%)`: in `echo %ERRORLEVEL%> file`, cmd reads `0>` as a redirect of handle 0 and
writes nothing.

A run takes about 9 minutes: the tray's first update check comes 5 minutes after it starts, and step 5 then waits 2
more minutes to see that the update does not apply while Keep awake holds. The update notice waits while the session
is locked, as every notice does; that check is then reported as SKIP.

.EXAMPLE
cargo build --locked --target x86_64-pc-windows-msvc -p wakeshot-tray -p wakeshot-cli
./packaging/e2e-update.ps1 -ExeDir target/x86_64-pc-windows-msvc/debug -WorkDir <new folder>

.EXAMPLE
./packaging/e2e-update.ps1 -ExeDir target/x86_64-pc-windows-msvc/debug -WorkDir <new folder> -PrepareOnly

.EXAMPLE
pwsh.exe -NoProfile -File <repo>\packaging\e2e-update.ps1 -ExeDir <exes> -WorkDir <new folder> > <log> 2>&1
(echo %ERRORLEVEL%)> <exit file>
#>
[CmdletBinding()]
param(
    # The folder that holds the x64 debug builds of wakeshot-tray.exe and wakeshot.exe.
    [Parameter(Mandatory)]
    [string] $ExeDir,

    # Packs, the feed, launch files, saved logs, BEFORE and the moved data folder go here. It must be empty or not
    # exist yet.
    [Parameter(Mandatory)]
    [string] $WorkDir,

    [string] $Icon = (Join-Path $PSScriptRoot '..\assets\wakeshot.ico'),

    # The vpk command; its version must match the velopack crate (1.2.161).
    [string] $Vpk = 'vpk',

    # Only the checks, the BEFORE read and the packs; nothing outside -WorkDir changes.
    [switch] $PrepareOnly,

    [switch] $AllowElevated,

    # Each install, uninstall and Setup run may take this long. Velopack's own hook limits are 30 s and 60 s.
    [int] $TimeoutSeconds = 120
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false

$PackId = 'HaikAsatryan.Wakeshot'
$Channel = 'win-x64'
$CopyName = 'Wakeshot int update'
$V1 = '0.0.1'
$V2 = '0.0.2'
$V3 = '0.0.3'
$LocalAppData = [Environment]::GetFolderPath('LocalApplicationData')
$Root = Join-Path $LocalAppData $PackId
$Current = Join-Path $Root 'current'
$TrayExe = Join-Path $Current 'wakeshot-tray.exe'
$CliExe = Join-Path $Current 'wakeshot.exe'
$UpdateExe = Join-Path $Root 'Update.exe'
$DataDir = Join-Path $LocalAppData 'Wakeshot'
$Journal = Join-Path $DataDir 'journal.json'
$StateFile = Join-Path $DataDir 'state.json'
$Holds = Join-Path $DataDir 'holds'
$Log = Join-Path $DataDir 'logs\wakeshot.log'
$Shortcut = Join-Path ([Environment]::GetFolderPath('Programs')) 'Wakeshot.lnk'
$VeloTemp = Join-Path ([IO.Path]::GetTempPath()) "velopack_$PackId"
$VelopackLogs = Join-Path $LocalAppData 'velopack'
$UninstallSub = "Software\Microsoft\Windows\CurrentVersion\Uninstall\$PackId"
$RunSub = 'Software\Microsoft\Windows\CurrentVersion\Run'
$RunOnceSub = 'Software\Microsoft\Windows\CurrentVersion\RunOnce'
$RunName = 'Wakeshot'
$RunOnceName = 'WakeshotRestore'
$EnvSub = 'Environment'
# The Run value the install hook writes, and the RunOnce value the tray writes while it holds (power/journal.rs).
$RunCommand = "`"$TrayExe`""
$RunOnceCommand = "`"$TrayExe`" --restore"
$GuidPattern = '[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}'
# `whoami /groups` names the Medium level by this SID.
$MediumSid = 'S-1-16-8192'
# update.rs: the first check comes after this much run time; the download may take a while more.
$FirstCheckSeconds = 300
$DownloadSeconds = 120
# How long step 5 watches the core keep a ready update while Keep awake holds.
$NoApplySeconds = 120
# Velopack waits 60 s for the tray to quit; a start through Explorer takes a few seconds before the tray runs.
$QuitSeconds = 90
$StartSeconds = 20
$FollowSeconds = 15

# RegQueryValueExW gives the bytes as stored; the .NET registry API trims trailing NULs, which would hide a change.
if (-not ('WakeshotE2eUpdate.Reg' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;

namespace WakeshotE2eUpdate {
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
$script:Skipped = 0
$script:Started = [Collections.Generic.List[object]]::new()
$script:Times = [ordered]@{}
$script:Launches = 0
$script:WrapperFiles = $null
$script:WrapperCopies = @()

function Say([string] $Text) { Write-Host $Text }

function Pass([string] $Check) { Say "PASS  $Check" }

function Fail([string] $Check, [string] $Why) {
    Say "FAIL  $Check`: $Why"
    $script:Failed++
}

# A check this run cannot make, with the reason; it neither passes nor fails the run.
function Skip([string] $Check, [string] $Why) {
    Say "SKIP  $Check`: $Why"
    $script:Skipped++
}

function Test-Check([string] $Check, [bool] $Ok, [string] $Why) {
    if ($Ok) { Pass $Check } else { Fail $Check $Why }
}

function Step([string] $Text) { Say "==== $((Get-Date).ToString('HH:mm:ss')) $Text" }

function Get-Hash([string] $Text) {
    [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($Text))).Substring(0, 16)
}

# Polls $Look until it gives something, at most $Seconds; gives that, or $null. Its own names are odd, since $Look
# reads the caller's variables through PowerShell's dynamic scope and a common name here would hide one of them.
function Wait-Until([double] $Seconds, [scriptblock] $Look, [int] $EveryMs = 500) {
    $waitUntilEnd = (Get-Date).AddSeconds($Seconds)
    while ($true) {
        $waitUntilGot = & $Look
        if ($waitUntilGot) { return $waitUntilGot }
        if ((Get-Date) -ge $waitUntilEnd) { return $null }
        Start-Sleep -Milliseconds $EveryMs
    }
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
    foreach ($name in $names) { $state[$name] = [WakeshotE2eUpdate.Reg]::Raw($Sub, $name) }
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

# A log line that names each value and a short hash of its stored bytes, not the value: the log may be shared in
# public.
function Format-KeyState($State) {
    $parts = foreach ($name in $State.Keys) { "$name=$(Get-Hash $State[$name])" }
    "$(@($State.Keys).Count) values: $($parts -join ', ')"
}

function Get-PathState {
    $type = [uint32]0
    $bytes = [WakeshotE2eUpdate.Reg]::Read($EnvSub, 'Path', [ref]$type)
    [pscustomobject]@{ Bytes = $bytes; Type = $type; Raw = [WakeshotE2eUpdate.Reg]::Raw($EnvSub, 'Path') }
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

# /qh, not /q: the test laptop hides the lid action and the energy saver threshold from /q.
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

# The user's plan is only read: a change to it means something wrote the wrong plan.
function Test-RealPlan([string] $Check) {
    $diff = @(Compare-PlanState $Before.Plan (Get-PlanState $Before.Active))
    Test-Check "$Check`: the user's plan's values unchanged" ($diff.Count -eq 0) ($diff -join '; ')
}

function Test-CopyActive([string] $Check) {
    $active = Get-ActivePlan
    Test-Check "$Check`: the copy is still the active plan" ($active -eq $script:Copy) "active is $active"
}

# The three values Keep awake writes on the active plan (e2e_real.rs, `hold`).
function Get-CopyKeys {
    $lid = Get-PlanValue $script:Copy 'SUB_BUTTONS' 'LIDACTION'
    $sleep = Get-PlanValue $script:Copy 'SUB_SLEEP' 'STANDBYIDLE'
    [pscustomobject]@{ LidAc = $lid.Ac; LidDc = $lid.Dc; SleepDc = $sleep.Dc }
}

function Format-CopyKeys($Keys) { "lid AC $($Keys.LidAc) DC $($Keys.LidDc), sleep DC $($Keys.SleepDc)" }

function Test-SameKeys($A, $B) { $A.LidAc -eq $B.LidAc -and $A.LidDc -eq $B.LidDc -and $A.SleepDc -eq $B.SleepDc }

function Test-KeysOurs($Keys) { $Keys.LidAc -eq 0 -and $Keys.LidDc -eq 0 -and $Keys.SleepDc -eq 0 }

# Delete sharing and a read that never fails: Wakeshot writes these files by rename, and a reader holds them open.
function Read-Json([string] $Path) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { return $null }
    try {
        $stream = [IO.File]::Open($Path, 'Open', 'Read', 'ReadWrite, Delete')
        try { $text = [IO.StreamReader]::new($stream).ReadToEnd() } finally { $stream.Dispose() }
        $text | ConvertFrom-Json
    } catch {
        $null
    }
}

# Within 15 s: Wakeshot's values on the copy, a journal of the three keys on the copy owned by $Tray, and the RunOnce
# value naming the installed tray.
function Test-Overridden([string] $Check, [Diagnostics.Process] $Tray) {
    $watch = [Diagnostics.Stopwatch]::StartNew()
    $null = Wait-Until $FollowSeconds { (Test-KeysOurs (Get-CopyKeys)) -and (Test-Path -LiteralPath $Journal) }
    $keys = Get-CopyKeys
    Test-Check "$Check`: Wakeshot's values on the copy (lid AC 0, lid DC 0, sleep DC 0)" (Test-KeysOurs $keys) "reads $(Format-CopyKeys $keys) after $FollowSeconds s"
    Say ('INFO  {0}: overridden {1:N1} s after the ask' -f $Check, $watch.Elapsed.TotalSeconds)
    $journal = Read-Json $Journal
    if ($null -eq $journal) {
        Fail "$Check`: journal.json owned by tray pid $($Tray.Id)" 'no journal.json, or it does not parse'
    } else {
        Test-Check "$Check`: journal.json owned by tray pid $($Tray.Id)" ($journal.owner.pid -eq $Tray.Id) "owner pid $($journal.owner.pid)"
        $entries = @($journal.entries)
        $names = (@($entries | ForEach-Object { "$($_.key)" } | Sort-Object) -join ',')
        $plans = (@($entries | ForEach-Object { "$($_.plan)".ToLowerInvariant() } | Sort-Object -Unique) -join ',')
        Test-Check "$Check`: journal entries lidAc, lidDc and sleepDc, all on the copy" (($names -ceq 'lidAc,lidDc,sleepDc') -and ($plans -eq $script:Copy)) "keys $names, plans $plans"
    }
    $runOnce = Get-RegValue $RunOnceSub $RunOnceName
    Test-Check "$Check`: RunOnce value $RunOnceName names the installed tray" ($runOnce -eq $RunOnceCommand) "'$runOnce'"
}

# Within $Seconds: the copy's own values, no journal and no RunOnce value, which is what every restore leaves.
function Test-CopyBack([string] $Check, [double] $Seconds = $FollowSeconds) {
    $null = Wait-Until $Seconds {
        (Test-SameKeys (Get-CopyKeys) $script:CopyKeys) -and -not (Test-Path -LiteralPath $Journal) -and
        ($null -eq (Get-RegValue $RunOnceSub $RunOnceName))
    }
    $keys = Get-CopyKeys
    Test-Check "$Check`: the copy's values are back ($(Format-CopyKeys $script:CopyKeys))" (Test-SameKeys $keys $script:CopyKeys) "reads $(Format-CopyKeys $keys)"
    Test-Check "$Check`: journal.json gone" (-not (Test-Path -LiteralPath $Journal)) "$Journal still there"
    Test-Check "$Check`: RunOnce value $RunOnceName gone" ($null -eq (Get-RegValue $RunOnceSub $RunOnceName)) "'$(Get-RegValue $RunOnceSub $RunOnceName)'"
}

# Every process this run starts, by pid and start time, so the end can tell it from a later process with the same pid.
# -Keep: never ended by this script (explorer.exe, which may hand over to the shell).
function Add-Started([Diagnostics.Process] $Process, [string] $What, [switch] $Keep) {
    $start = $Process.StartTime
    if (-not ($script:Started | Where-Object { $_.Pid -eq $Process.Id -and $_.Start -eq $start })) {
        $script:Started.Add([pscustomobject]@{ Pid = $Process.Id; Start = $start; What = $What; Keep = [bool]$Keep })
    }
}

# A handle kept open: the pid is never reused while it lives, and the exit code stays readable.
function Open-Process([int] $Id, [string] $What) {
    $process = [Diagnostics.Process]::GetProcessById($Id)
    $null = $process.Handle
    Add-Started $process $What
    $process
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
    Add-Started $process (Split-Path $Exe -Leaf)
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

# The installed CLI, as a user types it; it starts no child, so its output is read whole. Gives the exit code.
function Invoke-Cli([string] $Check, [string[]] $Arguments) {
    $info = [Diagnostics.ProcessStartInfo]::new($CliExe)
    foreach ($a in $Arguments) { $info.ArgumentList.Add($a) }
    $info.UseShellExecute = $false
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    $info.CreateNoWindow = $true
    $process = [Diagnostics.Process]::Start($info)
    Add-Started $process "wakeshot.exe $($Arguments -join ' ')"
    try {
        $out = $process.StandardOutput.ReadToEndAsync()
        $err = $process.StandardError.ReadToEndAsync()
        if (-not $process.WaitForExit(30000)) {
            $process.Kill($true)
            throw "wakeshot $($Arguments -join ' ') did not end within 30 s"
        }
        $null = $out.Wait(5000)
        $null = $err.Wait(5000)
        $said = (($out.Result + $err.Result).Trim() -split "`r?`n" | Where-Object { $_ }) -join ' | '
        Say "INFO  $Check`: wakeshot $($Arguments -join ' ') exit $($process.ExitCode): $said"
        $process.ExitCode
    } finally {
        $process.Dispose()
    }
}

function Start-Explorer([string] $File) {
    $info = [Diagnostics.ProcessStartInfo]::new((Join-Path $env:WINDIR 'explorer.exe'))
    $info.ArgumentList.Add($File)
    $info.UseShellExecute = $false
    $process = [Diagnostics.Process]::Start($info)
    try {
        Add-Started $process 'explorer.exe (launch)' -Keep
        # Explorer hands the file to the running shell; its exit code says nothing, the caller's wait is the check.
        $null = $process.WaitForExit(10000)
    } finally {
        $process.Dispose()
    }
}

# Processes started from the install folder.
function Get-InstalledProcesses {
    @(Get-CimInstance Win32_Process | Where-Object {
            $_.ExecutablePath -and $_.ExecutablePath.StartsWith("$Root\", [StringComparison]::OrdinalIgnoreCase)
        })
}

# The installed tray in tray mode: started with no argument, so not a guard (--guard) or a restore (--restore).
function Get-Trays {
    @(Get-CimInstance Win32_Process -Filter "Name = 'wakeshot-tray.exe'" | Where-Object {
            $_.ExecutablePath -and (Test-SamePath $_.ExecutablePath $TrayExe) -and $_.CommandLine -notmatch '\s--'
        })
}

# Every process from the install folder seen so far (guards, Velopack's restart, Update.exe) joins the list the end
# checks.
function Register-Installed {
    foreach ($p in @(Get-InstalledProcesses)) {
        $process = Get-Process -Id $p.ProcessId -ErrorAction SilentlyContinue
        if ($process) {
            $tail = ($p.CommandLine -replace '^\s*("[^"]*"|\S+)\s*', '').Trim()
            try { Add-Started $process "$($p.Name) $tail".Trim() } catch { Say "INFO  pid $($p.ProcessId) ended before it was listed" }
        }
    }
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

function Get-Lines([string] $Text) { @($Text -split "`r?`n" | Where-Object { $_ }) }

function Show-Lines([string] $Title, [string] $Text) {
    $lines = Get-Lines $Text
    Say "---- $Title ($($lines.Count) lines)"
    $lines | ForEach-Object { Say "  $_" }
}

# A line of Wakeshot's log (architecture section 8): `<time> [<seconds>] <process> <level> <area>: <text>`.
function Log-Line([string] $Process, [string] $Area, [string] $Text) {
    '\]\s' + $Process + '\s+INFO\s+' + [regex]::Escape($Area) + ': ' + [regex]::Escape($Text)
}

function Find-Line([string[]] $Lines, [string] $Pattern, [int] $From = 0) {
    for ($i = $From; $i -lt $Lines.Count; $i++) { if ($Lines[$i] -match $Pattern) { return $i } }
    -1
}

# Checks the lines a step added, in order: each pattern must match a line after the one before matched.
function Test-LogLines([string] $Check, [string] $Text, [string[]] $Patterns) {
    $lines = Get-Lines $Text
    $at = 0
    foreach ($pattern in $Patterns) {
        $hit = Find-Line $lines $pattern $at
        Test-Check "$Check`: log line /$pattern/" ($hit -ge 0) 'not found after the line before it in the lines this step added'
        if ($hit -ge 0) { $at = $hit + 1 }
    }
}

function Hook-Line([string] $Text) { Log-Line 'hook' 'hook' $Text }

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

# The lock screen runs LogonUI.exe in the session; while it shows, the core keeps every notice (core.rs, `notify`).
function Test-SessionLocked {
    $session = (Get-Process -Id $PID).SessionId
    [bool](Get-Process -Name 'LogonUI' -ErrorAction SilentlyContinue | Where-Object SessionId -eq $session)
}

function Get-SqVersion {
    $sq = Join-Path $Current 'sq.version'
    $text = if (Test-Path -LiteralPath $sq) { Get-Content -LiteralPath $sq -Raw } else { '' }
    if ($text -match '<version>([^<]+)</version>') { $Matches[1] } else { 'none' }
}

# What the feed folder lists, for the log: `releases.<channel>.json` is what Velopack's FileSource reads.
function Format-Feed {
    $index = Join-Path $script:Feed "releases.$Channel.json"
    try {
        $listed = @((Get-Content -LiteralPath $index -Raw | ConvertFrom-Json).Assets | ForEach-Object { "$($_.Version) $($_.Type)" }) -join ', '
    } catch {
        $listed = "unreadable: $($_.Exception.Message)"
    }
    $files = @(Get-ChildItem -LiteralPath $script:Feed -File | Sort-Object Name | ForEach-Object Name) -join ', '
    "lists $listed; files $files"
}

# A release as a feed holds it: the index Velopack reads and the packages, not Setup.exe or the portable zip.
function Copy-Release([string] $From) {
    foreach ($f in @(Get-ChildItem -LiteralPath $From -File)) {
        if ($f.Name -like '*.nupkg' -or $f.Name -ceq "releases.$Channel.json" -or $f.Name -ceq "RELEASES-$Channel") {
            Copy-Item -LiteralPath $f.FullName -Destination $script:Feed -Force
        }
    }
}

# Step 2: the three packs and the feed with the first one only. Each pack after the first gets the one before it, so
# 0.0.2 has a delta from 0.0.1, as a release has from the last one.
function New-Packs {
    $pack = Join-Path $PSScriptRoot 'pack.ps1'
    $script:PackDir = [ordered]@{}
    $previous = $null
    foreach ($v in $V1, $V2, $V3) {
        $script:PackDir[$v] = Join-Path $WorkDir "packs\$v"
        $packArgs = @{ Version = $v; Cpu = 'x64'; ExeDir = $ExeDir; OutDir = $script:PackDir[$v]; Vpk = $Vpk }
        if (Test-Path -LiteralPath $Icon -PathType Leaf) { $packArgs.Icon = $Icon }
        if ($previous) { $packArgs.PreviousDir = $previous }
        $watch = [Diagnostics.Stopwatch]::StartNew()
        $packText = & $pack @packArgs *>&1 | Out-String
        $packCode = $LASTEXITCODE
        $script:Times["pack $v"] = $watch.Elapsed.TotalSeconds
        Set-Content -LiteralPath (Join-Path $WorkDir "pack-$v.log") -Value $packText -Encoding utf8NoBOM
        @(Get-Lines $packText | Where-Object { $_ -match '^pack: ' }) | ForEach-Object { Say "  $_" }
        if ($packCode -ne 0) { throw "pack.ps1 for $v exited with $packCode (pack-$v.log)" }
        $previous = $script:PackDir[$v]
    }
    foreach ($v in $V1, $V2, $V3) {
        foreach ($f in @(Get-ChildItem -LiteralPath $script:PackDir[$v] -File | Sort-Object Name)) {
            Say ('INFO  step 2: {0,12:N0} bytes  {1}\{2}' -f $f.Length, $v, $f.Name)
        }
    }
    $script:Setup = @{}
    foreach ($v in $V1, $V3) {
        $script:Setup[$v] = Join-Path $script:PackDir[$v] "$PackId-$Channel-Setup.exe"
        Test-Check "step 2: $v Setup.exe" (Test-Path -LiteralPath $script:Setup[$v] -PathType Leaf) "$($script:Setup[$v]) not found"
    }
    $delta = Join-Path $script:PackDir[$V2] "$PackId-$V2-$Channel-delta.nupkg"
    Test-Check "step 2: $V2 has a delta from $V1" (Test-Path -LiteralPath $delta -PathType Leaf) "$delta not found"
    $script:Feed = Join-Path $WorkDir 'feed'
    New-Item -ItemType Directory -Force -Path $script:Feed | Out-Null
    Copy-Release $script:PackDir[$V1]
    Say "INFO  step 2: feed $($script:Feed): $(Format-Feed)"
}

# Starts the installed tray at Medium, as a user's start from the Start menu would, through explorer.exe and a .cmd
# that sets the feed, clears the data folder override and points Claude Code's folder at an empty one. Waits for the
# new tray and its first state.json, and gives it with a handle kept open.
function Start-Tray([string] $Check) {
    $script:Launches++
    $name = "tray-$($script:Launches)"
    $groups = Join-Path $WorkDir "$name-groups.txt"
    $launch = Join-Path $WorkDir "$name.cmd"
    $lines = @(
        '@echo off'
        'set "WAKESHOT_DATA_DIR="'
        "set `"WAKESHOT_UPDATE_FEED=$($script:Feed)`""
        "set `"CLAUDE_CONFIG_DIR=$($script:EmptyClaude)`""
        "whoami /groups > `"$groups`""
        "start `"`" `"$TrayExe`""
    )
    $text = ($lines -join "`r`n") + "`r`n"
    # cmd reads a batch file in the OEM code page; ASCII is the same in every one.
    if ($text -match '[^\x00-\x7F]') { throw "$launch would hold a path that is not ASCII; give a -WorkDir with an ASCII path" }
    [IO.File]::WriteAllText($launch, $text, [Text.Encoding]::ASCII)
    $launched = Get-Date
    $watch = [Diagnostics.Stopwatch]::StartNew()
    Start-Explorer $launch
    $found = Wait-Until $StartSeconds { Get-Trays | Where-Object { $_.CreationDate -ge $launched.AddSeconds(-1) } | Select-Object -First 1 }
    if (-not $found) { throw "$Check`: no tray from $TrayExe started within $StartSeconds s of the launch" }
    $tray = Open-Process $found.ProcessId 'wakeshot-tray.exe (tray)'
    $groupsText = Wait-Until 10 { if (Test-Path -LiteralPath $groups) { $t = Get-Content -LiteralPath $groups -Raw; if ($t -match 'S-1-16-') { $t } } }
    Test-Check "$Check`: the launch .cmd ran at Medium" ("$groupsText" -match $MediumSid) "no $MediumSid in $groups"
    $state = Wait-Until $FollowSeconds { $s = Read-Json $StateFile; if ($s -and $s.pid -eq $tray.Id) { $s } }
    Test-Check "$Check`: state.json names the new tray (pid $($tray.Id))" ([bool]$state) "not within $FollowSeconds s"
    $script:Times["tray start ($Check)"] = $watch.Elapsed.TotalSeconds
    Say ('INFO  {0}: tray pid {1} started from {2}; its state.json after {3:N1} s' -f $Check, $tray.Id, $launch, $watch.Elapsed.TotalSeconds)
    $tray
}

# The wrapper as a user runs it, its output kept in files: a pipe nobody reads would stop ping once it is full.
function Start-Wrapper {
    $info = [Diagnostics.ProcessStartInfo]::new($CliExe)
    foreach ($a in 'run', '--', 'ping', '-n', '600', '127.0.0.1') { $info.ArgumentList.Add($a) }
    $info.UseShellExecute = $false
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    $info.CreateNoWindow = $true
    $process = [Diagnostics.Process]::Start($info)
    $null = $process.Handle
    Add-Started $process 'wakeshot.exe run -- ping (wrapper)'
    $out = [IO.File]::Create((Join-Path $WorkDir 'wrapper-out.txt'))
    $err = [IO.File]::Create((Join-Path $WorkDir 'wrapper-err.txt'))
    $script:WrapperFiles = @($out, $err)
    $script:WrapperCopies = @(
        $process.StandardOutput.BaseStream.CopyToAsync($out),
        $process.StandardError.BaseStream.CopyToAsync($err))
    $process
}

# The copies end when the last writer (ping) ends.
function Close-WrapperFiles {
    if (-not $script:WrapperFiles) { return }
    foreach ($copy in $script:WrapperCopies) {
        # A copy that broke only loses output nobody checks.
        try { $null = $copy.Wait(5000) } catch { Say "INFO  the wrapper's output copy ended with: $($_.Exception.Message)" }
    }
    foreach ($file in $script:WrapperFiles) { $file.Dispose() }
    $script:WrapperFiles = $null
}

# ---- Refusals: nothing has changed yet.

$groups = (whoami /groups) -join "`n"
$level = if ($groups -match 'Mandatory Label\\(\w+) Mandatory Level') { $Matches[1] } else { 'unknown' }
$principal = [Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
$elevated = $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
$mode = if ($PrepareOnly) { 'prepare only' } else { 'full run' }
Say "e2e-update: $mode, user $env:USERNAME, integrity $level, elevated $elevated, pid $PID, $((Get-Date).ToString('yyyy-MM-dd HH:mm:ss'))"
if (($elevated -or $level -ne 'Medium') -and -not $AllowElevated -and -not $PrepareOnly) {
    Say 'e2e-update: refused: run it unelevated (Medium), as users run Wakeshot, or pass -AllowElevated on a CI runner.'
    exit 2
}

foreach ($exe in 'wakeshot-tray.exe', 'wakeshot.exe') {
    if (-not (Test-Path -LiteralPath (Join-Path $ExeDir $exe) -PathType Leaf)) {
        Say "e2e-update: refused: $(Join-Path $ExeDir $exe) not found"
        exit 2
    }
}
# velo.rs reads WAKESHOT_UPDATE_FEED in debug builds only; a release build does not even hold the name.
$trayBytes = [Text.Encoding]::Latin1.GetString([IO.File]::ReadAllBytes((Join-Path $ExeDir 'wakeshot-tray.exe')))
if ($trayBytes.IndexOf('WAKESHOT_UPDATE_FEED', [StringComparison]::Ordinal) -lt 0) {
    Say "e2e-update: refused: $(Join-Path $ExeDir 'wakeshot-tray.exe') does not read WAKESHOT_UPDATE_FEED; build it without --release"
    exit 2
}
$trayBytes = $null
if ((Test-Path -LiteralPath $WorkDir) -and (Get-ChildItem -LiteralPath $WorkDir -Force | Select-Object -First 1)) {
    Say "e2e-update: refused: $WorkDir is not empty; give a new or empty folder"
    exit 2
}

# Debug exes and their hooks read WAKESHOT_DATA_DIR (store.rs); this run is about the real data folder.
if ($env:WAKESHOT_DATA_DIR) {
    Say 'INFO  WAKESHOT_DATA_DIR was set; cleared for this run and the processes it starts'
    Remove-Item Env:WAKESHOT_DATA_DIR
}

$problems = [Collections.Generic.List[string]]::new()
$traces = @(Get-Traces)
if ($traces) { $problems.Add("Wakeshot is or was here: $($traces -join '; '). Uninstall it and remove these first.") }
if (Test-Path -LiteralPath $Journal) {
    $problems.Add("$Journal exists, so Wakeshot may own power settings now. Run ``wakeshot restore`` first.")
}
$running = @(Get-Process -Name 'wakeshot-tray', 'wakeshot' -ErrorAction SilentlyContinue)
if ($running) {
    $list = ($running | ForEach-Object { "pid $($_.Id) $($_.Path)" }) -join '; '
    $problems.Add("a Wakeshot process runs ($list); a second tray would stop at once on the tray mutex. Quit it first.")
}
$active = Get-ActivePlan
$plansNow = @(Get-Plans)
$activeName = @($plansNow | Where-Object Id -eq $active | ForEach-Object Name) -join ''
if ($activeName -like 'Wakeshot *') {
    $problems.Add("the active plan '$activeName' ($active) is a Wakeshot test copy, likely from a cut run. Switch to your own plan first.")
}
$leftovers = @($plansNow | Where-Object Name -ceq $CopyName)
if ($problems.Count) {
    foreach ($p in $problems) {
        if ($PrepareOnly) { Say "INFO  a full run would refuse: $p" } else { Say "e2e-update: refused: $p" }
    }
    if (-not $PrepareOnly) { exit 2 }
}
foreach ($p in @($plansNow | Where-Object { $_.Name -like 'Wakeshot *' -and $_.Name -cne $CopyName })) {
    Say "INFO  another card's plan copy is there and is left alone: $($p.Id) $($p.Name)"
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
    Plans = @($plansNow | Where-Object Name -cne $CopyName | ForEach-Object { "$($_.Id) $($_.Name)" })
    Plan = Get-PlanState $active
}
Say "INFO  Run: $(Format-KeyState $Before.Run)"
Say "INFO  RunOnce: $(Format-KeyState $Before.RunOnce)"
$pathLength = if ($null -eq $Before.Path.Bytes) { 'missing' } else { "$($Before.Path.Bytes.Length) bytes" }
Say "INFO  user Path: type $($Before.Path.Type), $pathLength, $(Get-Hash $Before.Path.Raw)"
Say "INFO  plans: $($Before.Plans -join '; '); active $active ($activeName)"
Say "INFO  active plan: $(Format-PlanState $Before.Plan)"
# Kept in the work folder only: the values themselves, for a restore by hand if this script is killed.
$Before | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $WorkDir 'before.json') -Encoding utf8NoBOM
Say "INFO  BEFORE saved to $(Join-Path $WorkDir 'before.json')"
Test-Check 'step 1: no HaikAsatryan.Wakeshot install' ($traces.Count -eq 0) ($traces -join '; ')

$movedData = $null
if ($PrepareOnly) {
    Say "INFO  step 1: prepare only, so $DataDir $(if (Test-Path -LiteralPath $DataDir) { 'exists and is not moved' } else { 'does not exist' })"
} elseif (Test-Path -LiteralPath $DataDir) {
    $movedData = Join-Path $WorkDir 'data-before\Wakeshot'
    New-Item -ItemType Directory -Force -Path (Split-Path $movedData) | Out-Null
    Move-Item -LiteralPath $DataDir -Destination $movedData
    Say "INFO  step 1: $DataDir moved to $movedData"
} else {
    Say "INFO  step 1: no $DataDir to move"
}

if ($PrepareOnly) {
    Step "step 2: pack $V1, $V2 and $V3; a feed with $V1 only"
    try {
        New-Packs
    } catch {
        Fail 'step 2' "stopped at line $($_.InvocationInfo.ScriptLineNumber): $($_.Exception.Message)"
    }
    $times = ($script:Times.Keys | ForEach-Object { '{0} {1:N1} s' -f $_, $script:Times[$_] }) -join ', '
    Say "INFO  times: $times"
    Say "e2e-update: prepare only: stopped after step 2; nothing outside $WorkDir changed but vpk's own work folder"
    if ($script:Failed) {
        Say "e2e-update: $script:Failed check(s) failed"
        exit 1
    }
    Say 'e2e-update: all checks passed'
    exit 0
}

$script:Copy = $null
$script:CopyKeys = $null
$script:Feed = $null
$script:EmptyClaude = Join-Path $WorkDir 'claude-empty'
$script:Wrapper = $null
$script:Ping = $null
$installedOnce = $false
try {
    # ---- Step 2: packs and the feed.

    Step "step 2: pack $V1, $V2 and $V3; a feed with $V1 only"
    New-Packs
    New-Item -ItemType Directory -Force -Path $script:EmptyClaude | Out-Null

    # ---- Step 3: the plan copy, made active.

    Step "step 3: plan copy '$CopyName', made active"
    foreach ($old in $leftovers) {
        $null = Invoke-Powercfg @('/delete', $old.Id)
        Say "INFO  step 3: deleted a leftover '$CopyName' copy $($old.Id) from a cut run"
    }
    $out = Invoke-Powercfg @('/duplicatescheme', $Before.Active)
    if ($out -notmatch "GUID:\s*($GuidPattern)") { throw "powercfg /duplicatescheme gave no plan: $out" }
    $script:Copy = $Matches[1].ToLowerInvariant()
    $null = Invoke-Powercfg @('/changename', $script:Copy, $CopyName)
    $script:CopyKeys = Get-CopyKeys
    Say "INFO  step 3: copy $($script:Copy): $(Format-CopyKeys $script:CopyKeys)"
    Test-Check 'step 3: copy listed under its name' ([bool](Get-Plans | Where-Object { $_.Id -eq $script:Copy -and $_.Name -ceq $CopyName })) 'not in powercfg /list'
    $zero = @(foreach ($k in 'LidAc', 'LidDc', 'SleepDc') { if ($script:CopyKeys.$k -eq 0) { $k } })
    Test-Check "step 3: none of the copy's lid AC, lid DC and sleep DC is 0 already" ($zero.Count -eq 0) "$($zero -join ', ') 0, so a restore could not be told apart"
    $null = Invoke-Powercfg @('/setactive', $script:Copy)
    Test-CopyActive 'step 3'
    Test-RealPlan 'step 3'

    # ---- Step 4: silent install of 0.0.1, then the tray.

    Step "step 4: Setup.exe --silent of $V1, then the installed tray at Medium"
    $logFrom = 0
    $code = Invoke-Exe $script:Setup[$V1] @('--silent')
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
    Test-Check "step 4: uninstall key DisplayVersion is $V1" ((Get-RegValue $UninstallSub 'DisplayVersion') -ceq $V1) "'$(Get-RegValue $UninstallSub 'DisplayVersion')'"
    $run = Get-RegValue $RunSub $RunName
    Test-Check "step 4: Run value $RunName is the quoted current\wakeshot-tray.exe" ($run -ceq $RunCommand) "'$run', want '$RunCommand'"
    Test-Check 'step 4: user Path has the current folder once' ((Get-CurrentInPath) -eq 1) "$(Get-CurrentInPath) entries"
    $text = Read-NewText $Log $logFrom
    $logFrom = Get-FileLength $Log
    Show-Lines "step 4: $Log, the install" $text
    Test-LogLines 'step 4' $text @(
        (Hook-Line "install hook for version $V1 started"),
        (Hook-Line "Run value $RunName set to $RunCommand"),
        # WARN when telling running programs failed, which this check does not judge.
        'hook\s+(INFO|WARN)\s+hook: CLI folder added to the user Path',
        (Hook-Line 'restore: no journal, nothing to restore'),
        (Hook-Line 'install hook done after'))
    $code = Invoke-Exe $CliExe @() -Quiet
    Test-Check 'step 4: wakeshot.exe with no arguments exits 0' ($code -eq 0) "exit $code"

    $trayFrom = Get-FileLength $Log
    $tray = Start-Tray 'step 4'
    $script:TrayStartedAt = $tray.StartTime
    $text = Read-NewText $Log $trayFrom
    Test-LogLines 'step 4' $text @(
        ('\]\stray\s+INFO\s+tray: Wakeshot \S+ starts: tray, pid ' + $tray.Id + '\b'),
        (Log-Line 'tray' 'tray' 'Claude Code is watched in the folder CLAUDE_CONFIG_DIR names'),
        (Log-Line 'tray' 'tray' 'Wakeshot is installed, so it checks for updates'))
    $state = Read-Json $StateFile
    Test-Check 'step 4: nothing holds the new tray' ($state -and @($state.holding).Count -eq 0) "holding: $(if ($state) { @($state.holding) -join ', ' })"
    Test-Check "step 4: the copy's values are its own" (Test-SameKeys (Get-CopyKeys) $script:CopyKeys) "reads $(Format-CopyKeys (Get-CopyKeys))"
    Register-Installed
    Test-CopyActive 'step 4'
    Test-RealPlan 'step 4'

    # ---- Step 5: an update waits for the hold, then applies.

    Step "step 5: Keep awake 30m; $V2 in the feed; the update waits, then applies after keep-awake off"
    $updateLogStep = Get-FileLength $updateLog
    $code = Invoke-Cli 'step 5' @('keep-awake', '30m')
    Test-Check 'step 5: wakeshot keep-awake 30m exits 0' ($code -eq 0) "exit $code"
    Test-Overridden 'step 5' $tray
    Register-Installed
    Copy-Release $script:PackDir[$V2]
    Say "INFO  step 5: feed now $(Format-Feed)"
    $due = $script:TrayStartedAt.AddSeconds($FirstCheckSeconds + $DownloadSeconds)
    Say "INFO  step 5: the first check is due at $($script:TrayStartedAt.AddSeconds($FirstCheckSeconds).ToString('HH:mm:ss')); waiting until $($due.ToString('HH:mm:ss')) at most"
    $downloaded = Log-Line 'tray' 'update' "update $V2 downloaded"
    $readyLine = Log-Line 'tray' 'core' "Update $V2 is ready"
    $seen = Wait-Until (($due - (Get-Date)).TotalSeconds) {
        $t = Read-NewText $Log $trayFrom
        if (($t -match $downloaded) -and ($t -match $readyLine)) { Get-Date }
    } -EveryMs 2000
    if (-not $seen) {
        Show-Lines "step 5: $Log since the tray started" (Read-NewText $Log $trayFrom)
        throw "the tray did not download $V2 by $($due.ToString('HH:mm:ss'))"
    }
    $script:Times['first check, from the tray start'] = ($seen - $script:TrayStartedAt).TotalSeconds
    Pass "step 5: the first check downloaded $V2 and the core keeps it ($('{0:N0}' -f $script:Times['first check, from the tray start']) s after the start)"
    $readyFrom = Get-FileLength $Log
    Say "INFO  step 5: watching $NoApplySeconds s for an apply that must not come"
    Start-Sleep -Seconds $NoApplySeconds
    $text = Read-NewText $Log $readyFrom
    Test-Check "step 5: no apply in the $NoApplySeconds s after the update was ready" ($text -notmatch 'Applying update') ((Get-Lines $text | Where-Object { $_ -match 'update' }) -join ' | ')
    Test-Check "step 5: tray pid $($tray.Id) still runs" (-not $tray.HasExited) "it ended with $(if ($tray.HasExited) { $tray.ExitCode })"
    Test-Check "step 5: Wakeshot's values still on the copy" (Test-KeysOurs (Get-CopyKeys)) "reads $(Format-CopyKeys (Get-CopyKeys))"
    $currentBorn = (Get-Item -LiteralPath $Current).CreationTimeUtc

    $offFrom = Get-FileLength $Log
    $offAt = Get-Date
    $watch = [Diagnostics.Stopwatch]::StartNew()
    $code = Invoke-Cli 'step 5' @('keep-awake', 'off')
    Test-Check 'step 5: wakeshot keep-awake off exits 0' ($code -eq 0) "exit $code"
    Test-CopyBack 'step 5'
    $script:Times['restore after keep-awake off'] = $watch.Elapsed.TotalSeconds
    $quit = $tray.WaitForExit($QuitSeconds * 1000)
    $script:Times['old tray exit after keep-awake off'] = $watch.Elapsed.TotalSeconds
    Test-Check "step 5: the old tray (pid $($tray.Id)) quits" $quit "still runs $QuitSeconds s after keep-awake off"
    if ($quit) { Test-Check 'step 5: the old tray quits with code 0' ($tray.ExitCode -eq 0) "code $($tray.ExitCode)" }
    $found = Wait-Until $QuitSeconds { Get-Trays | Where-Object { $_.CreationDate -gt $offAt } | Select-Object -First 1 }
    if (-not $found) {
        Show-Lines "step 5: $Log since keep-awake off" (Read-NewText $Log $offFrom)
        throw "no new tray from $TrayExe within $QuitSeconds s of the old one's exit"
    }
    $tray = Open-Process $found.ProcessId 'wakeshot-tray.exe (tray, Velopack restart)'
    $script:Times['new tray after keep-awake off'] = $watch.Elapsed.TotalSeconds
    $restartedLine = Log-Line 'tray' 'tray' "Velopack restarted the tray after the update to $V2"
    $null = Wait-Until 30 { (Read-NewText $Log $offFrom) -match $restartedLine }
    $state = Wait-Until $FollowSeconds { $s = Read-Json $StateFile; if ($s -and $s.pid -eq $tray.Id) { $s } }
    Test-Check "step 5: state.json names the new tray (pid $($tray.Id))" ([bool]$state) "not within $FollowSeconds s"
    $noticeLine = '\]\stray\s+(INFO|WARN)\s+io: notice (shown: Updated|Updated not shown)'
    $null = Wait-Until 30 { (Read-NewText $Log $offFrom) -match $noticeLine }
    $text = Read-NewText $Log $offFrom
    Show-Lines "step 5: $Log since keep-awake off" $text
    $lines = Get-Lines $text
    # Any line naming the copy after keep-awake off is the restore's; nothing else writes the copy then.
    $restoreAt = Find-Line $lines ([regex]::Escape($script:Copy))
    $applyAt = Find-Line $lines (Log-Line 'tray' 'core' "Applying update $V2")
    Test-Check 'step 5: the copy was put back before the core applied the update' (($restoreAt -ge 0) -and ($applyAt -gt $restoreAt)) "first line naming the copy at $restoreAt, apply at $applyAt"
    Test-LogLines 'step 5' $text @(
        (Log-Line 'tray' 'core' "Applying update $V2"),
        (Log-Line 'tray' 'update' 'the updater started; it applies the update once Wakeshot quits'),
        (Log-Line 'tray' 'core' 'Update applied: Wakeshot quits so the updater can finish'),
        (Log-Line 'tray' 'tray' 'Wakeshot ends with code 0'),
        (Hook-Line "obsolete hook for version $V1"),
        (Hook-Line "updated hook for version $V2 started"),
        (Hook-Line 'restore: no journal, nothing to restore'),
        (Hook-Line 'updated hook done after'),
        ('\]\stray\s+INFO\s+tray: Wakeshot \S+ starts: tray, pid ' + $tray.Id + '\b'),
        (Log-Line 'tray' 'tray' 'Wakeshot is installed, so it checks for updates'),
        $restartedLine)
    # Velopack starts the new tray from the old one's Update.exe; whether the .cmd's variables reach it is recorded.
    $claude = if ($text -match (Log-Line 'tray' 'tray' 'Claude Code is watched in the folder CLAUDE_CONFIG_DIR names')) { 'the empty folder CLAUDE_CONFIG_DIR names' } else { '.claude of the profile folder' }
    Say "INFO  step 5: the restarted tray watches $claude"
    if ($text -match (Log-Line 'tray' 'io' 'notice shown: Updated')) {
        Pass 'step 5: the update notice shows'
    } elseif ($text -match 'notice Updated not shown') {
        Fail 'step 5: the update notice shows' ((Get-Lines $text | Where-Object { $_ -match 'notice Updated not shown' }) -join ' | ')
    } elseif (Test-SessionLocked) {
        Skip 'step 5: the update notice shows' 'the session is locked (LogonUI.exe runs in it), and the core keeps every notice until the unlock (core.rs, notify)'
    } else {
        Fail 'step 5: the update notice shows' 'no notice line within 30 s, and the session is not locked'
    }
    $warned = @(Get-Lines $text | Where-Object { $_ -match '\]\s\w+\s+(WARN|ERROR)\s' })
    Say "INFO  step 5: $($warned.Count) WARN or ERROR lines since keep-awake off$(if ($warned) { ': ' + ($warned -join ' | ') })"
    Test-Check "step 5: current\sq.version is $V2" ((Get-SqVersion) -ceq $V2) "$(Get-SqVersion)"
    Test-Check "step 5: uninstall key DisplayVersion is $V2" ((Get-RegValue $UninstallSub 'DisplayVersion') -ceq $V2) "'$(Get-RegValue $UninstallSub 'DisplayVersion')'"
    Test-Check 'step 5: current keeps its path' (Test-Path -LiteralPath $TrayExe -PathType Leaf) "$TrayExe not found"
    $newBorn = if (Test-Path -LiteralPath $Current) { (Get-Item -LiteralPath $Current).CreationTimeUtc } else { $null }
    Say "INFO  step 5: current created $($currentBorn.ToString('o')) before, $(if ($newBorn) { $newBorn.ToString('o') } else { 'none' }) after"
    Test-Check "step 5: Run value $RunName unchanged" ((Get-RegValue $RunSub $RunName) -ceq $RunCommand) "'$(Get-RegValue $RunSub $RunName)'"
    Test-Check 'step 5: user Path has the current folder once' ((Get-CurrentInPath) -eq 1) "$(Get-CurrentInPath) entries"
    Show-Lines "step 5: $updateLog, this step" (Read-NewText $updateLog $updateLogStep)
    Register-Installed
    Test-CopyActive 'step 5'
    Test-RealPlan 'step 5'

    # ---- Step 6: Setup.exe over the install during a hold, as winget upgrade runs it.

    Step "step 6: Keep awake on and a wakeshot run wrapper, then Setup.exe --silent of $V3 over the install"
    $code = Invoke-Cli 'step 6' @('keep-awake', 'on')
    Test-Check 'step 6: wakeshot keep-awake on exits 0' ($code -eq 0) "exit $code"
    Test-Overridden 'step 6' $tray
    $script:Wrapper = Start-Wrapper
    $found = Wait-Until 10 { Get-CimInstance Win32_Process -Filter "ParentProcessId = $($script:Wrapper.Id) AND Name = 'PING.EXE'" | Select-Object -First 1 }
    if (-not $found) { throw "the wrapper (pid $($script:Wrapper.Id)) started no PING.EXE within 10 s" }
    $script:Ping = Open-Process $found.ProcessId 'PING.EXE (the wrapper''s command)'
    $holdFile = Join-Path $Holds "run-$($script:Ping.Id).json"
    $state = Wait-Until $FollowSeconds { $s = Read-Json $StateFile; if ((Test-Path -LiteralPath $holdFile) -and $s -and @($s.holding).Count -ge 2) { $s } }
    Test-Check "step 6: hold file run-$($script:Ping.Id).json names the ping" (Test-Path -LiteralPath $holdFile) "$holdFile not found"
    Test-Check 'step 6: the tray holds for Keep awake and the wrapper' ([bool]$state) "holding: $(@((Read-Json $StateFile).holding) -join ', ')"
    Say "INFO  step 6: wrapper pid $($script:Wrapper.Id), ping pid $($script:Ping.Id), tray pid $($tray.Id)"
    Register-Installed
    $setupFrom = Get-FileLength $Log
    $code = Invoke-Exe $script:Setup[$V3] @('--silent')
    $script:Times['setup over the install'] = $script:LastSeconds
    Test-Check 'step 6: Setup over the install exits 0' ($code -eq 0) "exit $code"
    Say ('INFO  step 6: Setup over the install took {0:N1} s' -f $script:LastSeconds)
    # Wakeshot stays off after winget upgrade until the next sign-in, so nothing from the install folder may run now.
    Test-NoneStarted 'step 6'
    Test-CopyBack 'step 6'
    Test-Check "step 6: the tray (pid $($tray.Id)) ended" $tray.HasExited 'still runs'
    $wrapperEnded = $script:Wrapper.WaitForExit(5000)
    Test-Check 'step 6: the wrapper ended, as the README says' $wrapperEnded 'still runs'
    if ($wrapperEnded) { Say "INFO  step 6: the wrapper ended with code $($script:Wrapper.ExitCode)" }
    Test-Check 'step 6: its ping goes on, as the README says' (-not $script:Ping.HasExited) "ended with $(if ($script:Ping.HasExited) { $script:Ping.ExitCode })"
    if (-not $script:Ping.HasExited) {
        Stop-Process -Id $script:Ping.Id -Force
        $null = $script:Ping.WaitForExit(5000)
        Say "INFO  step 6: ping pid $($script:Ping.Id) ended by its pid"
    }
    Close-WrapperFiles
    $text = Read-NewText $Log $setupFrom
    Show-Lines "step 6: $Log, this step" $text
    Test-LogLines 'step 6' $text @(
        (Hook-Line "install hook for version $V3 started"),
        '\]\shook\s+INFO\s+hook: restore: (the journal''s values are back|no journal, nothing to restore)',
        (Hook-Line 'install hook done after'))
    $who = @(Get-Lines $text | Where-Object { $_ -match [regex]::Escape($script:Copy) } | ForEach-Object { if ($_ -match '\]\s(\w+)\s') { $Matches[1] } } | Sort-Object -Unique)
    Say "INFO  step 6: lines naming the copy came from: $(if ($who) { $who -join ', ' } else { 'no process' })"
    Test-Check "step 6: current\sq.version is $V3" ((Get-SqVersion) -ceq $V3) "$(Get-SqVersion)"
    Test-Check "step 6: uninstall key DisplayVersion is $V3" ((Get-RegValue $UninstallSub 'DisplayVersion') -ceq $V3) "'$(Get-RegValue $UninstallSub 'DisplayVersion')'"
    Test-Check "step 6: Run value $RunName unchanged" ((Get-RegValue $RunSub $RunName) -ceq $RunCommand) "'$(Get-RegValue $RunSub $RunName)'"
    Test-Check 'step 6: user Path has the current folder once' ((Get-CurrentInPath) -eq 1) "$(Get-CurrentInPath) entries"
    Test-CopyActive 'step 6'
    Test-RealPlan 'step 6'

    # ---- Step 7: an uninstall during a hold restores and removes everything.

    Step 'step 7: the tray again, Keep awake on, then the uninstall command'
    $tray = Start-Tray 'step 7'
    $code = Invoke-Cli 'step 7' @('keep-awake', 'on')
    Test-Check 'step 7: wakeshot keep-awake on exits 0' ($code -eq 0) "exit $code"
    Test-Overridden 'step 7' $tray
    Register-Installed
    $text = Read-NewText $Log 0
    Set-Content -LiteralPath (Join-Path $WorkDir 'wakeshot-before-uninstall.log') -Value $text -Encoding utf8NoBOM
    Say "INFO  step 7: Settings > Apps runs UninstallString '$(Get-RegValue $UninstallSub 'UninstallString')'; this step runs the silent one"
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
    Test-Check "step 7: the tray (pid $($tray.Id)) ended" $tray.HasExited 'still runs'
    $keys = Get-CopyKeys
    Test-Check "step 7: the copy's values are back ($(Format-CopyKeys $script:CopyKeys))" (Test-SameKeys $keys $script:CopyKeys) "reads $(Format-CopyKeys $keys)"
    Test-Check "step 7: Run value $RunName gone" ($null -eq (Get-RegValue $RunSub $RunName)) 'still there'
    Test-Check "step 7: RunOnce value $RunOnceName gone" ($null -eq (Get-RegValue $RunOnceSub $RunOnceName)) 'still there'
    Test-Check 'step 7: user Path entry gone' ((Get-CurrentInPath) -eq 0) "$Current still in Path"
    Test-Check 'step 7: user Path has its old type and bytes' ((Get-PathState).Raw -ceq $Before.Path.Raw) 'differs from BEFORE'
    # The uninstall deletes the data folder after a good restore.
    Test-Check 'step 7: data folder gone' (Wait-Gone $DataDir 5) "$DataDir still there"
    Test-Check 'step 7: install folder gone' $gone "$Root still there"
    Test-Check 'step 7: uninstall key gone' (-not (Test-RegKey $UninstallSub)) "HKCU\$UninstallSub still there"
    Test-Check 'step 7: Start menu shortcut gone' (-not (Test-Path -LiteralPath $Shortcut)) "$Shortcut still there"
    Test-Check "step 7: $VeloTemp gone" (Wait-Gone $VeloTemp 5) 'still there'
    Test-CopyActive 'step 7'
    Test-RealPlan 'step 7'
} catch {
    Fail 'run' "stopped at line $($_.InvocationInfo.ScriptLineNumber): $($_.Exception.Message)"
} finally {
    # ---- Step 8: AFTER. Each part runs even when one before it fails.

    Step 'step 8: AFTER'
    $tidy = @(
        {
            if ((Test-RegKey $UninstallSub) -or (Test-Path -LiteralPath $Root)) {
                Register-Installed
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
            foreach ($p in @($script:Wrapper, $script:Ping)) {
                if ($p -and -not $p.HasExited) {
                    Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
                    $null = $p.WaitForExit(5000)
                    Fail 'step 8: the wrapper or its ping still ran' "ended pid $($p.Id) by its pid"
                }
            }
            Close-WrapperFiles
        },
        {
            # Refused at the start when either was there, so any value now is this run's.
            foreach ($v in @(@($RunSub, $RunName), @($RunOnceSub, $RunOnceName))) {
                if ($null -ne (Get-RegValue $v[0] $v[1])) {
                    [WakeshotE2eUpdate.Reg]::Delete($v[0], $v[1])
                    Fail "step 8: $($v[1]) was still there" 'removed now'
                }
            }
        },
        {
            if ((Get-PathState).Raw -cne $Before.Path.Raw) {
                if ($null -eq $Before.Path.Bytes) {
                    [WakeshotE2eUpdate.Reg]::Delete($EnvSub, 'Path')
                } else {
                    [WakeshotE2eUpdate.Reg]::Write($EnvSub, 'Path', $Before.Path.Type, $Before.Path.Bytes)
                }
                $null = [WakeshotE2eUpdate.Reg]::Broadcast()
                Fail 'step 8: user Path differed from BEFORE' 'BEFORE bytes written back'
            }
        },
        {
            # A tray applies to the active plan, so the user's plan waits until none of this run's can run.
            $left = @(Get-InstalledProcesses)
            if ($left) {
                Fail "step 8: the user's plan is not made active" "a process from the install folder still runs: $(($left | ForEach-Object { "pid $($_.ProcessId)" }) -join ', ')"
                return
            }
            $now = Get-ActivePlan
            if ($now -ne $Before.Active) {
                $null = Invoke-Powercfg @('/setactive', $Before.Active)
                Say "INFO  step 8: the user's plan $($Before.Active) made active again (was $now)"
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
                Fail "step 8: the user's plan's values differed from BEFORE" "written back: $($writes -join '; ')"
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
        Test-Check "step 8: the user's plan's values equal BEFORE" ($diff.Count -eq 0) ($diff -join '; ')
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
        foreach ($s in $alive) { if (-not $s.Keep) { Stop-Process -Id $s.Pid -Force -ErrorAction SilentlyContinue } }
        $aliveText = ($alive | ForEach-Object { "pid $($_.Pid) $($_.What)$(if ($_.Keep) { ' (left running)' } else { ' (stopped)' })" }) -join ', '
        Test-Check "step 8: none of the $($script:Started.Count) processes this run started is left" ($alive.Count -eq 0) $aliveText
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
    Say "e2e-update: $script:Failed check(s) failed, $script:Skipped skipped"
    exit 1
}
Say "e2e-update: all checks passed, $script:Skipped skipped"
exit 0
