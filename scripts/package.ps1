<#
.SYNOPSIS
    Puts the built viewer and its files in a folder ready to launch (or to
    zip): exe, assets, licences. The GitHub release and build-release.ps1
    both use it: their folders are identical. The Linux job of the release
    runs it too (PowerShell 7), with the executable without extension.

.EXAMPLE
    ./scripts/package.ps1 -Exe target/release/aurora-viewer.exe -Out Aurora-Viewer
.EXAMPLE
    ./scripts/package.ps1 -Exe target/release/aurora-viewer -Out Aurora-Viewer-v0.8.0-linux-x64
#>
param(
    [Parameter(Mandatory)][string]$Exe,
    [Parameter(Mandatory)][string]$Out,
    # repository whose assets go with the exe (default: this script's)
    [string]$Source = (Split-Path $PSScriptRoot -Parent)
)

New-Item -ItemType Directory -Force -Path (Join-Path $Out 'assets/emoji') -ErrorAction Stop | Out-Null
Copy-Item -LiteralPath $Exe -Destination $Out -ErrorAction Stop
# Ship approved artwork alongside the executable. Fail packaging if absent.
$logoEncoded = Join-Path $Source 'assets/branding/waters-roux-logo.png.base64'
if (-not (Test-Path -LiteralPath $logoEncoded)) {
    throw "Missing Waters Roux artwork: $logoEncoded"
}
$brandingOut = Join-Path $Out 'assets/branding'
New-Item -ItemType Directory -Force -Path $brandingOut -ErrorAction Stop | Out-Null
$logoBytes = [Convert]::FromBase64String(([IO.File]::ReadAllText($logoEncoded)).Trim())
if ($logoBytes.Length -lt 24 -or $logoBytes[0] -ne 137 -or $logoBytes[1] -ne 80) {
    throw "Invalid Waters Roux logo asset"
}
[IO.File]::WriteAllBytes((Join-Path $brandingOut 'waters-roux-logo.png'), $logoBytes)
# The emoji font is inside the exe; its licence ships, and the assets folder
# stays: the updater of the published versions refuses a package without it.
Copy-Item -LiteralPath (Join-Path $Source 'assets/emoji/LICENSE-OFL.txt') -Destination (Join-Path $Out 'assets/emoji') -ErrorAction Stop
# The spelling dictionaries are inside the exe too: their licences ship
# (GPL, LGPL and MPL ask for the text), one per dictionary, with the list of
# their sources.
$dictionaries = Join-Path $Source 'crates/aurora-viewer/assets/dictionaries'
$dictionariesOut = Join-Path $Out 'assets/dictionaries'
New-Item -ItemType Directory -Force -Path $dictionariesOut -ErrorAction Stop | Out-Null
Copy-Item -LiteralPath (Join-Path $dictionaries 'README.md') -Destination $dictionariesOut -ErrorAction Stop
Get-ChildItem -LiteralPath $dictionaries -Directory -ErrorAction Stop | ForEach-Object {
    $license = Join-Path $_.FullName 'LICENSE.txt'
    if (Test-Path -LiteralPath $license) {
        Copy-Item -LiteralPath $license -Destination (Join-Path $dictionariesOut "$($_.Name)-LICENSE.txt") -ErrorAction Stop
    }
}
$docs = 'LICENSE', 'NOTICE.md', 'README.md' | ForEach-Object { Join-Path $Source $_ }
Copy-Item -LiteralPath $docs -Destination $Out -ErrorAction Stop
# Linux (an executable without extension): the desktop entry and its icon,
# named after the viewer's Wayland app_id / X11 WM_CLASS (cli.rs DESKTOP_ID)
# so the desktop matches the window to them. docs/LINUX.md installs them.
# (macOS has its own bundle: package-macos.ps1; $IsMacOS is unset, so
# false, in Windows PowerShell 5.1.)
if (-not [IO.Path]::GetExtension($Exe) -and -not $IsMacOS) {
    Copy-Item -LiteralPath (Join-Path $Source 'packaging/linux/aurora-viewer.desktop') -Destination $Out -ErrorAction Stop
    Copy-Item -LiteralPath (Join-Path $Source 'assets/branding/aurora-logo.svg') -Destination (Join-Path $Out 'aurora-viewer.svg') -ErrorAction Stop
}
$taskExePath = (Resolve-Path -LiteralPath $Exe).Path
$taskVersion = (& $taskExePath --version | Out-String).Trim() -replace '^Aurora Viewer ', ''
if ($LASTEXITCODE -ne 0 -or $taskVersion -notmatch '^\d+\.\d+\.\d+([+-][0-9A-Za-z.+-]+)?$') { throw 'Invalid package version.' }
$taskJson = @{ schema = 1; version = $taskVersion } | ConvertTo-Json
$taskMetadataPath = Join-Path (Resolve-Path -LiteralPath $Out -ErrorAction Stop).Path 'aurora-install.json'
# Windows PowerShell 5.1 does not support Set-Content -Encoding utf8NoBOM.
[IO.File]::WriteAllText($taskMetadataPath, $taskJson, [Text.UTF8Encoding]::new($false))
