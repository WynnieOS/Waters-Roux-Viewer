<# Portable development packaging, never an installer or production release. #>
param(
    [Parameter(Mandatory)][string]$Exe,
    [Parameter(Mandatory)][string]$Out
)
$ErrorActionPreference = 'Stop'
$source = Split-Path $PSScriptRoot -Parent
$logo = Join-Path $source 'assets/branding/waters-roux-logo.png'
$icon = Join-Path $source 'assets/branding/waters-roux.ico'
if (-not (Test-Path $logo) -or -not (Test-Path $icon)) {
    Write-Warning 'Original logo/icon files are missing. The preview will use text branding and the system application icon; no substitute logo.'
}
if (Test-Path -LiteralPath $Out) { throw 'Choose a new output folder; existing previews are never overwritten.' }
& (Join-Path $PSScriptRoot 'package.ps1') -Exe $Exe -Out $Out
Move-Item -LiteralPath (Join-Path $Out 'aurora-viewer.exe') -Destination (Join-Path $Out 'waters-roux-viewer.exe')
if (Test-Path $icon) { Copy-Item -LiteralPath $icon -Destination (Join-Path $Out 'assets/branding/waters-roux.ico') }
'Waters Roux portable development profile v1' | Set-Content -LiteralPath (Join-Path $Out 'waters-roux-portable.txt') -Encoding utf8
@'
@echo off
cd /d "%~dp0"
if not exist "profile" mkdir "profile"
if not exist "profile\write-test.tmp" echo portable>"profile\write-test.tmp"
if errorlevel 1 (
  echo Extract the complete preview into a writable folder before launching.
  pause
  exit /b 1
)
del "profile\write-test.tmp"
start "Waters Roux Viewer" "waters-roux-viewer.exe" %*
'@ | Set-Content -LiteralPath (Join-Path $Out 'Launch Waters Roux.cmd') -Encoding ascii
@'
WATERS ROUX VIEWER — PORTABLE DEVELOPMENT BETA

1. Extract the complete ZIP into a NEW writable folder, for example
   Documents\Waters-Roux-Beta-<build>. Do not run inside the ZIP.
2. Double-click Launch Waters Roux.cmd (or waters-roux-viewer.exe).
3. Log in yourself using the normal account, grid and start-location controls.
4. Preferences > Appearance > Skin > Glass opacity: 10–100%, applied live.
   Preferences > Accessibility > High Contrast provides opaque panels.
5. Close and reopen to check saved preferences.

The profile folder beside this executable holds this preview's settings,
cache and account metadata. Keep the whole preview folder together. A new
extracted folder starts a separate profile; no earlier preview is modified.
Password hashes use the OS credential vault under Waters Roux's beta identity.
No installer or automatic update is included. This is not a production release.

Background artwork: if no waters-roux-background.png is packaged, the login
uses a temporary procedural bayou dusk illustration. The reference screenshot
is never used as a background and its sample data is never displayed.

Built on Aurora Viewer. LICENSE, NOTICE.md, dictionary and emoji licenses
are included. Source: https://github.com/WynnieOS/Waters-Roux-Viewer
The build commit and exact version are in BUILD-INFO.txt.
'@ | Set-Content -LiteralPath (Join-Path $Out 'START-HERE.txt') -Encoding utf8
$version = (& (Join-Path (Resolve-Path $Out).Path 'waters-roux-viewer.exe') --version | Out-String).Trim()
if ($LASTEXITCODE -ne 0 -or $version -notmatch '^Waters Roux Viewer ') { throw 'Packaged executable is not Waters Roux branded.' }
$commit = (git -C $source rev-parse HEAD | Out-String).Trim()
"$version`nSource commit: $commit`nDevelopment preview only." | Set-Content -LiteralPath (Join-Path $Out 'BUILD-INFO.txt') -Encoding utf8
if (Test-Path $logo) {
    $originalHash = (Get-FileHash $logo -Algorithm SHA256).Hash
    $packagedHash = (Get-FileHash (Join-Path $Out 'assets/branding/waters-roux-logo.png') -Algorithm SHA256).Hash
    if ($originalHash -ne $packagedHash) { throw 'The original logo was altered during packaging.' }
    "Logo SHA256: $originalHash" | Add-Content -LiteralPath (Join-Path $Out 'BUILD-INFO.txt') -Encoding utf8
} else {
    'Missing artwork: original Waters Roux logo PNG and derived ICO. Text branding only; no substitute logo.' | Set-Content -LiteralPath (Join-Path $Out 'MISSING-ARTWORK.txt') -Encoding utf8
}
