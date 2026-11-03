#Requires -Version 7
<#
.SYNOPSIS
Writes the winget manifest of one Wakeshot release: the three files in templates/, filled with the version, the
release date and the SHA-256 of both Setup.exe files.

.DESCRIPTION
By default the Setup.exe files are downloaded from the tag's published GitHub release and hashed, so the manifest
holds the hashes of exactly the files winget will download (docs/research/launch.md, section 2, rule 3). With
-FromDir the hashes come from local files instead, for a check before the release exists; the URLs are always the
release's. Only normal releases go to winget, so a pre-release version or a GitHub pre-release is refused (rule 6).
GH_TOKEN, when set, is sent to the GitHub API only (a higher rate limit), never to the downloads.

.EXAMPLE
./packaging/winget/fill.ps1 -Version 0.1.0 -OutDir out/winget
#>
[CmdletBinding()]
param(
    # The release version, three numbers: the tag without its "v".
    [Parameter(Mandatory)]
    [string] $Version,

    # Where the three manifest files go. It must be new or empty: `wingetcreate submit` sends every file in it.
    [Parameter(Mandatory)]
    [string] $OutDir,

    # A folder holding both HaikAsatryan.Wakeshot-win-<cpu>-Setup.exe files, used instead of the release.
    [string] $FromDir,

    # yyyy-MM-dd. Default: the release's publish date (UTC); with -FromDir and no date, the field is left out.
    [string] $ReleaseDate
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$Repo = 'HaikAsatryan/wakeshot'
$PackId = 'HaikAsatryan.Wakeshot'
$Cpus = 'x64', 'arm64'
$Templates = Join-Path $PSScriptRoot 'templates'

function Fail([string] $Why) {
    Write-Host "fill: $Why"
    exit 1
}

function Get-SetupName([string] $Cpu) {
    "$PackId-win-$Cpu-Setup.exe"
}

# Three numbers, no leading zeros and no pre-release part: winget sees only the three numbers Velopack writes to
# DisplayVersion, so a second spelling of one version would loop `winget upgrade` (research/launch.md, rule 2).
if ($Version -notmatch '^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$') {
    Fail "version '$Version' is not three numbers; pre-releases never go to winget"
}
$parsed = [datetime]::MinValue
$invariant = [Globalization.CultureInfo]::InvariantCulture
if ($ReleaseDate -and -not [datetime]::TryParseExact($ReleaseDate, 'yyyy-MM-dd', $invariant, 'None', [ref]$parsed)) {
    Fail "release date '$ReleaseDate' is not yyyy-MM-dd"
}
if ((Test-Path -LiteralPath $OutDir) -and (Get-ChildItem -LiteralPath $OutDir -Force | Select-Object -First 1)) {
    Fail "$OutDir is not empty; give a new or empty folder"
}

$hashes = @{}
if ($FromDir) {
    foreach ($cpu in $Cpus) {
        $path = Join-Path $FromDir (Get-SetupName $cpu)
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { Fail "$path not found" }
        $hashes[$cpu] = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash
    }
} else {
    $headers = @{ Accept = 'application/vnd.github+json'; 'X-GitHub-Api-Version' = '2022-11-28' }
    if ($env:GH_TOKEN) { $headers.Authorization = "Bearer $env:GH_TOKEN" }
    $api = "https://api.github.com/repos/$Repo/releases/tags/v$Version"
    try {
        $release = Invoke-RestMethod -Uri $api -Headers $headers
    } catch {
        Fail "no published release v$Version ($api): $($_.Exception.Message)"
    }
    if ($release.draft) { Fail "release v$Version is still a draft" }
    if ($release.prerelease) { Fail "release v$Version is a pre-release; pre-releases never go to winget" }
    if (-not $ReleaseDate) { $ReleaseDate = ([datetime]$release.published_at).ToUniversalTime().ToString('yyyy-MM-dd') }

    $download = Join-Path ([IO.Path]::GetTempPath()) "wakeshot-winget-$([guid]::NewGuid().ToString('N'))"
    New-Item -ItemType Directory -Path $download | Out-Null
    try {
        foreach ($cpu in $Cpus) {
            $name = Get-SetupName $cpu
            $asset = @($release.assets | Where-Object name -ceq $name)
            if ($asset.Count -ne 1) { Fail "release v$Version has no asset $name" }
            # The public URL, the same one the manifest gives winget.
            $url = "https://github.com/$Repo/releases/download/v$Version/$name"
            $file = Join-Path $download $name
            Invoke-WebRequest -Uri $url -OutFile $file
            $hash = (Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash
            # GitHub keeps its own SHA-256 of each asset; a mismatch means the download is not the uploaded file.
            $digest = $asset[0].PSObject.Properties['digest']
            if ($digest -and $digest.Value -and $digest.Value -ne "sha256:$($hash.ToLowerInvariant())") {
                Fail "$name downloaded with SHA-256 $hash, but GitHub lists $($digest.Value)"
            }
            $hashes[$cpu] = $hash
        }
    } finally {
        Remove-Item -LiteralPath $download -Recurse -Force -ErrorAction SilentlyContinue
    }
}

$values = @{
    '{{VERSION}}' = $Version
    '{{X64_SHA256}}' = $hashes['x64']
    '{{ARM64_SHA256}}' = $hashes['arm64']
    '{{RELEASE_DATE}}' = $ReleaseDate
}

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$utf8 = [Text.UTF8Encoding]::new($false)
$templateFiles = @(Get-ChildItem -LiteralPath $Templates -File -Filter '*.yaml')
if ($templateFiles.Count -ne 3) { Fail "want 3 templates in $Templates, found $($templateFiles.Count)" }
foreach ($template in $templateFiles) {
    $lines = [IO.File]::ReadAllLines($template.FullName, $utf8)
    if (-not $ReleaseDate) { $lines = @($lines | Where-Object { $_ -notmatch '^ReleaseDate:' }) }
    $text = ($lines -join "`n") + "`n"
    foreach ($key in $values.Keys) {
        if ($text.Contains($key)) { $text = $text.Replace($key, $values[$key]) }
    }
    if ($text -match '\{\{[A-Z0-9_]+\}\}') { Fail "$($template.Name) still has $($Matches[0])" }
    [IO.File]::WriteAllText((Join-Path $OutDir $template.Name), $text, $utf8)
}

Write-Host "fill: $PackId $Version, release date $(if ($ReleaseDate) { $ReleaseDate } else { 'left out' })"
foreach ($cpu in $Cpus) { Write-Host "fill: $cpu $($hashes[$cpu])" }
Write-Host "fill: manifest in $((Resolve-Path -LiteralPath $OutDir).Path)"
