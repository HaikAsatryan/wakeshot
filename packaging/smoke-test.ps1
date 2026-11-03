#Requires -Version 7
<#
.SYNOPSIS
Installs a Wakeshot Setup.exe silently, checks what the install left, uninstalls it silently and checks that
everything is gone. One line per check; exit 0 when all pass, 1 when any fails, 2 when it refuses to run.

.DESCRIPTION
Runs the same two commands winget and Apps and Features run: `Setup.exe --silent`, then the uninstall key's
QuietUninstallString. Setup never starts the tray when silent, so nothing holds the PC awake during the test.

The script never touches power settings. It refuses to run when any trace of Wakeshot is already there: the uninstall
deletes the data folder, and a journal in it would make the install and uninstall hooks restore power settings. So run
it only for a user where Wakeshot was never installed: a CI runner, or a test account.

.EXAMPLE
./packaging/smoke-test.ps1 -Setup out/x64/HaikAsatryan.Wakeshot-win-x64-Setup.exe -Version 0.1.0
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [string] $Setup,

    # The version packed into Setup.exe, as given to pack.ps1.
    [Parameter(Mandatory)]
    [string] $Version,

    # Each of the install and the uninstall may take this long. Velopack's own hook limits are 30 s and 60 s.
    [int] $TimeoutSeconds = 120
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false

$PackId = 'HaikAsatryan.Wakeshot'
$LocalAppData = [Environment]::GetFolderPath('LocalApplicationData')
$Root = Join-Path $LocalAppData $PackId
$Current = Join-Path $Root 'current'
$TrayExe = Join-Path $Current 'wakeshot-tray.exe'
$DataDir = Join-Path $LocalAppData 'Wakeshot'
$HookLog = Join-Path $DataDir 'logs\wakeshot.log'
$Shortcut = Join-Path ([Environment]::GetFolderPath('Programs')) 'Wakeshot.lnk'
$UninstallSub = "Software\Microsoft\Windows\CurrentVersion\Uninstall\$PackId"
$RunSub = 'Software\Microsoft\Windows\CurrentVersion\Run'
$RunOnceSub = 'Software\Microsoft\Windows\CurrentVersion\RunOnce'
$RunName = 'Wakeshot'
$RunOnceName = 'WakeshotRestore'
$EnvSub = 'Environment'
$VelopackLogs = Join-Path $LocalAppData 'velopack'
# Velopack writes only major.minor.patch to DisplayVersion (`get_manifest_version_short_string`), and hooks get the
# whole version.
$ShortVersion = ($Version -split '-', 2)[0]

# RegQueryValueExW gives the bytes as stored; the .NET registry API trims trailing NULs, which would hide a change.
if (-not ('WakeshotSmoke.Reg' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;

namespace WakeshotSmoke {
    public static class Reg {
        [DllImport("advapi32.dll", CharSet = CharSet.Unicode)]
        private static extern int RegOpenKeyExW(IntPtr key, string subKey, uint options, uint access, out IntPtr result);

        [DllImport("advapi32.dll", CharSet = CharSet.Unicode)]
        private static extern int RegQueryValueExW(IntPtr key, string name, IntPtr reserved, out uint type, byte[] data, ref uint size);

        [DllImport("advapi32.dll")]
        private static extern int RegCloseKey(IntPtr key);

        private const int NotFound = 2;
        private const int MoreData = 234;

        public static string Raw(string subKey, string name) {
            // HKEY_CURRENT_USER is sign-extended on 64-bit Windows, as the SDK headers define it.
            IntPtr hkcu = new IntPtr(unchecked((int)0x80000001));
            IntPtr key;
            int rc = RegOpenKeyExW(hkcu, subKey, 0, 0x0001, out key);
            if (rc == NotFound) return "missing";
            if (rc != 0) throw new Win32Exception(rc);
            try {
                while (true) {
                    uint type;
                    uint size = 0;
                    rc = RegQueryValueExW(key, name, IntPtr.Zero, out type, null, ref size);
                    if (rc == NotFound) return "missing";
                    if (rc != 0) throw new Win32Exception(rc);
                    byte[] data = new byte[size];
                    rc = RegQueryValueExW(key, name, IntPtr.Zero, out type, data, ref size);
                    // The value grew between the two calls.
                    if (rc == MoreData) continue;
                    if (rc != 0) throw new Win32Exception(rc);
                    return "type " + type + ", " + size + " bytes: " + BitConverter.ToString(data, 0, (int)size);
                }
            } finally {
                RegCloseKey(key);
            }
        }
    }
}
'@
}

$script:Failed = 0

function Pass([string] $Check) {
    Write-Host "PASS  $Check"
}

function Fail([string] $Check, [string] $Why) {
    Write-Host "FAIL  $Check`: $Why"
    $script:Failed++
}

function Test-Check([string] $Check, [bool] $Ok, [string] $Why) {
    if ($Ok) { Pass $Check } else { Fail $Check $Why }
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

function Get-PathEntries {
    $path = Get-RegValue $EnvSub 'Path'
    if ($null -eq $path) { return @() }
    @($path -split ';' | Where-Object { $_ } | ForEach-Object { [Environment]::ExpandEnvironmentVariables($_).TrimEnd('\') })
}

function Test-PathHasCurrent {
    $want = $Current.TrimEnd('\')
    [bool](Get-PathEntries | Where-Object { [string]::Equals($_, $want, [StringComparison]::OrdinalIgnoreCase) })
}

function Get-ShortcutTarget {
    (New-Object -ComObject WScript.Shell).CreateShortcut($Shortcut).TargetPath
}

function Test-SamePath([string] $A, [string] $B) {
    [string]::Equals($A.TrimEnd('\'), $B.TrimEnd('\'), [StringComparison]::OrdinalIgnoreCase)
}

# Runs an exe and waits; a run past the timeout is killed by its PID and reported as such. -Quiet keeps a console
# exe's output out of the one-line-per-check report; it is only for exes that start no child, since a child that
# inherits the pipe would keep the read open.
function Invoke-Exe([string] $Exe, [string[]] $Arguments, [switch] $Quiet) {
    $info = [Diagnostics.ProcessStartInfo]::new($Exe)
    foreach ($a in $Arguments) { $info.ArgumentList.Add($a) }
    $info.UseShellExecute = $false
    $info.RedirectStandardOutput = [bool]$Quiet
    $process = [Diagnostics.Process]::Start($info)
    try {
        $output = if ($Quiet) { $process.StandardOutput.ReadToEndAsync() }
        if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
            $process.Kill($true)
            return "timed out after $TimeoutSeconds s"
        }
        if ($output) { $null = $output.Wait(5000) }
        $process.ExitCode
    } finally {
        $process.Dispose()
    }
}

function Wait-Gone([string] $Path, [int] $Seconds) {
    # Update.exe cannot delete itself, so the install folder goes about 3 s after it exits (research.md, "Velopack
    # checks for Wakeshot", Uninstall).
    $until = (Get-Date).AddSeconds($Seconds)
    while ((Test-Path -LiteralPath $Path) -and (Get-Date) -lt $until) { Start-Sleep -Milliseconds 500 }
    -not (Test-Path -LiteralPath $Path)
}

function Get-FileLength([string] $Path) {
    if (Test-Path -LiteralPath $Path -PathType Leaf) { (Get-Item -LiteralPath $Path).Length } else { 0 }
}

# Velopack keeps appending to its logs across installs, so only what this run added is shown.
function Show-NewLines([string] $Path, [long] $From) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { return }
    $stream = [IO.File]::Open($Path, 'Open', 'Read', 'ReadWrite, Delete')
    try {
        if ($From -gt $stream.Length) { $From = 0 }
        $stream.Position = $From
        $text = [IO.StreamReader]::new($stream).ReadToEnd()
    } finally {
        $stream.Dispose()
    }
    $lines = @($text -split "`r?`n" | Where-Object { $_ })
    if ($lines) {
        Write-Host "---- $Path, this run"
        $lines | Select-Object -Last 60 | ForEach-Object { Write-Host "  $_" }
    }
}

if (-not (Test-Path -LiteralPath $Setup -PathType Leaf)) {
    Write-Host "smoke-test: $Setup not found"
    exit 2
}

$traces = @()
if (Test-Path -LiteralPath $Root) { $traces += "install folder $Root" }
if (Test-Path -LiteralPath $DataDir) { $traces += "data folder $DataDir" }
if (Test-RegKey $UninstallSub) { $traces += "uninstall key HKCU\$UninstallSub" }
if ($null -ne (Get-RegValue $RunSub $RunName)) { $traces += "Run value $RunName" }
if ($null -ne (Get-RegValue $RunOnceSub $RunOnceName)) { $traces += "RunOnce value $RunOnceName" }
if (Test-PathHasCurrent) { $traces += "user Path entry $Current" }
if (Test-Path -LiteralPath $Shortcut) { $traces += "shortcut $Shortcut" }
if ($traces) {
    Write-Host "smoke-test: refused, Wakeshot was here before: $($traces -join '; ')."
    Write-Host 'smoke-test: the uninstall deletes the data folder, and a journal there makes the hooks restore power settings.'
    exit 2
}

$pathBefore = [WakeshotSmoke.Reg]::Raw($EnvSub, 'Path')
$hookLogText = $null
$setupLog = Join-Path $VelopackLogs 'velopack.log'
$updateLog = Join-Path $VelopackLogs "velopack_$PackId.log"
$setupLogFrom = Get-FileLength $setupLog
$updateLogFrom = Get-FileLength $updateLog

Write-Host "smoke-test: $Setup, version $Version, user $env:USERNAME"
$installed = $false
try {
    $code = Invoke-Exe (Resolve-Path -LiteralPath $Setup).Path @('--silent')
    $installed = $true
    Test-Check 'silent install exits 0' ($code -eq 0) "exit $code"

    foreach ($file in 'Update.exe', 'Wakeshot.exe', 'current\wakeshot-tray.exe', 'current\wakeshot.exe', 'current\sq.version') {
        $path = Join-Path $Root $file
        Test-Check "file $file" (Test-Path -LiteralPath $path -PathType Leaf) "$path not found"
    }

    if (Test-Path -LiteralPath $Shortcut) {
        $target = Get-ShortcutTarget
        Test-Check 'Start menu shortcut' (Test-SamePath $target $TrayExe) "$Shortcut points at '$target', want $TrayExe"
    } else {
        Fail 'Start menu shortcut' "$Shortcut not found"
    }

    if (Test-RegKey $UninstallSub) {
        $name = Get-RegValue $UninstallSub 'DisplayName'
        $publisher = Get-RegValue $UninstallSub 'Publisher'
        $shown = Get-RegValue $UninstallSub 'DisplayVersion'
        $quiet = Get-RegValue $UninstallSub 'QuietUninstallString'
        Test-Check 'uninstall key DisplayName' ($name -ceq 'Wakeshot') "'$name'"
        Test-Check 'uninstall key Publisher' ($publisher -ceq 'Haik Asatryan') "'$publisher'"
        # winget compares this with the manifest's PackageVersion (research/launch.md, section 2, rule 5).
        Test-Check "uninstall key DisplayVersion is $ShortVersion" ($shown -ceq $ShortVersion) "'$shown'"
        Test-Check 'uninstall key QuietUninstallString' ([bool]$quiet) 'missing'
    } else {
        Fail 'uninstall key' "HKCU\$UninstallSub not found"
    }

    $run = Get-RegValue $RunSub $RunName
    $wantRun = "`"$TrayExe`""
    Test-Check "Run value $RunName" ($null -ne $run -and [string]::Equals($run, $wantRun, [StringComparison]::OrdinalIgnoreCase)) "'$run', want '$wantRun'"

    Test-Check 'user Path entry' (Test-PathHasCurrent) "no entry $Current"

    if (Test-Path -LiteralPath $HookLog -PathType Leaf) {
        $hookLogText = Get-Content -LiteralPath $HookLog -Raw
        # A line from log.rs: `<time> [<clock>] hook    INFO  hook: install hook for version <v> started`. vpk does not
        # check that the tray calls VelopackApp, so this line is the only proof the hooks run (research.md, point 10).
        $line = 'hook\s+INFO\s+hook: install hook for version ' + [regex]::Escape($Version) + ' started'
        Test-Check 'install hook log line' ($hookLogText -match $line) "no line like '$line' in $HookLog"
    } else {
        Fail 'install hook log line' "$HookLog not found"
    }

    # The winget pipeline runs installed exes with no arguments and flags a non-zero exit (research/distribution.md,
    # section 2). The tray is not run: it would start Wakeshot for real.
    $cli = Join-Path $Current 'wakeshot.exe'
    if (Test-Path -LiteralPath $cli -PathType Leaf) {
        $code = Invoke-Exe $cli @() -Quiet
        Test-Check 'wakeshot.exe with no arguments exits 0' ($code -eq 0) "exit $code"
    }
} finally {
    if ($installed -or (Test-RegKey $UninstallSub) -or (Test-Path -LiteralPath $Root)) {
        $quiet = Get-RegValue $UninstallSub 'QuietUninstallString'
        if ($quiet -match '^"([^"]+)"\s*(.*)$') {
            $exe = $Matches[1]
            $rest = @($Matches[2] -split '\s+' | Where-Object { $_ })
        } else {
            $exe = Join-Path $Root 'Update.exe'
            $rest = @('--uninstall', '--silent')
            Write-Host "smoke-test: no usable QuietUninstallString ('$quiet'), so $exe $($rest -join ' ')"
        }
        if (Test-Path -LiteralPath $exe -PathType Leaf) {
            $code = Invoke-Exe $exe $rest
            Test-Check 'silent uninstall exits 0' ($code -eq 0) "exit $code"
        } else {
            Fail 'silent uninstall exits 0' "$exe not found"
        }
    }
}

Test-Check 'install folder gone' (Wait-Gone $Root 30) "$Root still there"
Test-Check 'uninstall key gone' (-not (Test-RegKey $UninstallSub)) "HKCU\$UninstallSub still there"
Test-Check "Run value $RunName gone" ($null -eq (Get-RegValue $RunSub $RunName)) 'still there'
Test-Check "RunOnce value $RunOnceName gone" ($null -eq (Get-RegValue $RunOnceSub $RunOnceName)) 'still there'
Test-Check 'user Path entry gone' (-not (Test-PathHasCurrent)) "$Current still in Path"
$pathAfter = [WakeshotSmoke.Reg]::Raw($EnvSub, 'Path')
Test-Check 'user Path has its old bytes' ($pathAfter -ceq $pathBefore) "before: $pathBefore; after: $pathAfter"
Test-Check 'Start menu shortcut gone' (-not (Test-Path -LiteralPath $Shortcut)) "$Shortcut still there"
# Question 2 of the ship track: the data folder goes after a good restore, and a clean user has nothing to restore.
Test-Check 'data folder gone' (Wait-Gone $DataDir 5) "$DataDir still there"

if ($script:Failed) {
    if ($hookLogText) {
        Write-Host "---- $HookLog as it was after the install"
        $hookLogText -split "`r?`n" | Where-Object { $_ } | ForEach-Object { Write-Host "  $_" }
    }
    Show-NewLines $setupLog $setupLogFrom
    Show-NewLines $updateLog $updateLogFrom
    # The data folder was not there before the run, so all of a log left in it is this run's.
    Show-NewLines $HookLog 0
    Write-Host "smoke-test: $script:Failed check(s) failed"
    exit 1
}
Write-Host 'smoke-test: all checks passed'
exit 0
