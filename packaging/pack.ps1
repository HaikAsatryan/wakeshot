#Requires -Version 7

<#
.SYNOPSIS
Packs one Velopack release of Wakeshot for one CPU: Setup.exe, the full package, a delta when an earlier release is
given, the portable zip and the release index.

.DESCRIPTION
Copies only wakeshot-tray.exe and wakeshot.exe into a staging folder and runs `vpk pack` with the values in
docs/phases/06-ship.md, "Package and install values". Stops on the first failure with a non-zero exit code. Used the
same way here and in .github/workflows/release.yml; packaging/README.md says how to run it.

.EXAMPLE
./packaging/pack.ps1 -Version 0.1.0 -Cpu x64 -ExeDir target/x86_64-pc-windows-msvc/release -OutDir out/x64 -Icon assets/wakeshot.ico
#>
[CmdletBinding()]
param(
    # The release version, SemVer 2 without build metadata: the tag without its "v".
    [Parameter(Mandatory)]
    [string] $Version,

    [Parameter(Mandatory)]
    [ValidateSet('x64', 'arm64')]
    [string] $Cpu,

    # The folder that holds the release builds of wakeshot-tray.exe and wakeshot.exe for this CPU.
    [Parameter(Mandatory)]
    [string] $ExeDir,

    # Where the release files go. It must be empty or not exist yet, so a stale file can never pass for a new one.
    [Parameter(Mandatory)]
    [string] $OutDir,

    # The icon of Setup.exe and Update.exe.
    [string] $Icon,

    # A folder with earlier full packages (`vpk download github` writes one). vpk builds the delta against the newest
    # one of this CPU. A folder that does not exist means there is no earlier release.
    [string] $PreviousDir,

    # The vpk command; its version must match the velopack crate (1.2.161).
    [string] $Vpk = 'vpk'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
# Exit codes of native commands are checked by hand below, the same way on every PowerShell 7 version.
$PSNativeCommandUseErrorActionPreference = $false

$PackId = 'HaikAsatryan.Wakeshot'
$Channel = "win-$Cpu"
$Exes = 'wakeshot-tray.exe', 'wakeshot.exe'
# The PE header's Machine field: a mixed-up exe folder must not ship x64 code in the ARM64 channel.
$Machine = @{ x64 = 0x8664; arm64 = 0xAA64 }[$Cpu]

function Fail([string] $Why) {
    Write-Host "pack: $Why"
    exit 1
}

function Get-PeMachine([string] $Path) {
    $bytes = [IO.File]::ReadAllBytes($Path)
    if ($bytes.Length -lt 0x40 -or $bytes[0] -ne 0x4D -or $bytes[1] -ne 0x5A) { return $null }
    $pe = [BitConverter]::ToInt32($bytes, 0x3C)
    if ($pe -lt 0 -or $pe + 6 -gt $bytes.Length) { return $null }
    if ([BitConverter]::ToUInt32($bytes, $pe) -ne 0x00004550) { return $null }
    [BitConverter]::ToUInt16($bytes, $pe + 4)
}

# Three numbers, then an optional pre-release part. Build metadata (`+...`) is refused: NuGet drops it, so two builds
# would share one package version.
if ($Version -notmatch '^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(-[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?$') {
    Fail "version '$Version' is not SemVer 2 without build metadata (for example 0.1.0 or 0.1.0-rc.1)"
}

foreach ($exe in $Exes) {
    $path = Join-Path $ExeDir $exe
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { Fail "$path not found" }
    $found = Get-PeMachine $path
    if ($found -ne $Machine) {
        $seen = if ($null -eq $found) { 'unreadable' } else { '0x{0:X4}' -f $found }
        Fail ('{0} is not a {1} exe (PE machine {2}, want 0x{3:X4})' -f $path, $Cpu, $seen, $Machine)
    }
}

if ($Icon -and -not (Test-Path -LiteralPath $Icon -PathType Leaf)) { Fail "icon $Icon not found" }

# vpk reads every option it has from a VPK_* variable too (`vpk pack -H`), so a stray one would change the package
# without showing up in the command line below.
$stray = @(Get-ChildItem Env: | Where-Object Name -like 'VPK_*' | ForEach-Object Name)
if ($stray) { Fail "$($stray -join ', ') set; vpk would read it as a pack option. Unset it first" }

if ((Test-Path -LiteralPath $OutDir) -and (Get-ChildItem -LiteralPath $OutDir -Force | Select-Object -First 1)) {
    Fail "$OutDir is not empty; give a new or empty folder"
}
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$OutDir = (Resolve-Path -LiteralPath $OutDir).Path

if ($PreviousDir -and (Test-Path -LiteralPath $PreviousDir -PathType Container)) {
    $earlier = @(Get-ChildItem -LiteralPath $PreviousDir -File -Filter "$PackId-*-$Channel-full.nupkg")
    foreach ($file in $earlier) { Copy-Item -LiteralPath $file.FullName -Destination $OutDir }
    Write-Host "pack: $($earlier.Count) earlier full package(s) of $Channel from $PreviousDir"
} else {
    Write-Host 'pack: no earlier release, so no delta'
}

$staging = Join-Path ([IO.Path]::GetTempPath()) "wakeshot-pack-$([guid]::NewGuid().ToString('N'))"
New-Item -ItemType Directory -Path $staging | Out-Null
try {
    foreach ($exe in $Exes) { Copy-Item -LiteralPath (Join-Path $ExeDir $exe) -Destination $staging }

    # -r is always given: without it vpk packs for x86 (research/distribution.md, section 1).
    $vpkArgs = @(
        'pack'
        '--packId', $PackId
        '--packVersion', $Version
        '--runtime', $Channel
        '--channel', $Channel
        '--packDir', $staging
        '--mainExe', 'wakeshot-tray.exe'
        '--packTitle', 'Wakeshot'
        '--packAuthors', 'Haik Asatryan'
        '--shortcuts', 'StartMenuRoot'
        '--outputDir', $OutDir
    )
    if ($Icon) { $vpkArgs += '--icon', (Resolve-Path -LiteralPath $Icon).Path }

    Write-Host "pack: $Vpk $($vpkArgs -join ' ')"
    & $Vpk @vpkArgs
    if ($LASTEXITCODE -ne 0) { Fail "vpk pack exited with $LASTEXITCODE" }
} finally {
    Remove-Item -LiteralPath $staging -Recurse -Force -ErrorAction SilentlyContinue
}

$want = @(
    "$PackId-$Channel-Setup.exe"
    "$PackId-$Version-$Channel-full.nupkg"
    "$PackId-$Channel-Portable.zip"
    "releases.$Channel.json"
    "assets.$Channel.json"
)
foreach ($name in $want) {
    if (-not (Test-Path -LiteralPath (Join-Path $OutDir $name) -PathType Leaf)) { Fail "vpk wrote no $name" }
}

Get-ChildItem -LiteralPath $OutDir -File | Sort-Object Name | ForEach-Object {
    Write-Host ('pack: {0,12:N0} bytes  {1}' -f $_.Length, $_.Name)
}
Write-Host "pack: $PackId $Version for $Channel is in $OutDir"
