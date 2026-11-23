<# Waters Roux portable development packaging for Beta 27, Beta 28 V3, or v1.0; never a production release. #>
param(
    [Parameter(Mandatory)][string]$Exe,
    [Parameter(Mandatory)][string]$Out,
    [ValidateSet('Beta 27', 'Beta 28 V3', 'v1.0 Development')][string]$BetaLabel = 'Beta 27'
)
$ErrorActionPreference = 'Stop'
$previewFolderLabel = if ($BetaLabel -eq 'v1.0 Development') { 'V1-0' } elseif ($BetaLabel -eq 'Beta 28 V3') { 'Beta-28-V3' } else { 'Beta-27' }
$acceptanceSource = if ($BetaLabel -eq 'v1.0 Development') {
    'docs/WATERS_ROUX_V1_INTEGRATION_PLAN.md'
} elseif ($BetaLabel -eq 'Beta 28 V3') {
    'docs/WATERS_ROUX_BETA28_V3_ACCEPTANCE.md'
} else {
    'docs/WATERS_ROUX_BETA27.md'
}
$acceptanceLabel = if ($BetaLabel -eq 'v1.0 Development') { 'V1-0-ACCEPTANCE.md' } elseif ($BetaLabel -eq 'Beta 28 V3') { 'BETA-28-V3-ACCEPTANCE.md' } else { 'BETA-27-ACCEPTANCE.md' }
$source = Split-Path $PSScriptRoot -Parent
$logo = Join-Path $source 'assets/branding/waters-roux-logo.png'
$icon = Join-Path $source 'assets/branding/waters-roux.ico'
if (-not (Test-Path $logo) -or -not (Test-Path $icon)) {
    Write-Warning 'Original logo/icon files are missing. The preview will use text branding and the system application icon; no substitute logo.'
}
if (Test-Path -LiteralPath $Out) { throw 'Choose a new output folder; existing previews are never overwritten.' }
& (Join-Path $PSScriptRoot 'package.ps1') -Exe $Exe -Out $Out
New-Item -ItemType Directory -Force -Path (Join-Path $Out 'assets/branding') | Out-Null
Move-Item -LiteralPath (Join-Path $Out 'aurora-viewer.exe') -Destination (Join-Path $Out 'waters-roux-viewer.exe')
if (Test-Path $icon) { Copy-Item -LiteralPath $icon -Destination (Join-Path $Out 'assets/branding/waters-roux.ico') }
'Waters Roux portable development profile v1' | Set-Content -LiteralPath (Join-Path $Out 'waters-roux-portable.txt') -Encoding utf8
$launchScript = @'
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
start "Waters Roux Viewer - Beta 27" "waters-roux-viewer.exe" %*
'@
$launchScript.Replace('Beta 27', $BetaLabel) | Set-Content -LiteralPath (Join-Path $Out 'Launch Waters Roux.cmd') -Encoding ascii
$startHere = @'
WATERS ROUX VIEWER — BETA 27 — PORTABLE DEVELOPMENT PREVIEW

1. Extract the complete ZIP into a NEW writable folder, for example
   Documents\Waters-Roux-Viewer-Beta-27-<build>. Do not run inside the ZIP.
2. Double-click Launch Waters Roux.cmd (or waters-roux-viewer.exe).
3. Log in yourself using the normal account, grid and start-location controls.
4. Before login, open the gear button for Login Appearance / Branding.
   In the world, open Preferences > Waters Roux > Appearance & Branding.
5. Use Login or Loading to choose independent background and logo images.
   Use Glass for 10–100% opacity and its supported visual controls.
   Preferences > Accessibility > High Contrast provides opaque panels.
6. Choose Image opens the native file picker. Backgrounds accept PNG/JPEG/GIF;
   logos accept PNG. Inspect the preview, then Apply. Cancel discards the
   pending choices. Reset restores the current section; Defaults restores
   the complete official preset. Official artwork may be missing: see below.
7. Close and reopen to check saved preferences and selections.

CUSTOM ARTWORK STAYS LOCAL
The viewer imports personal images into its own local profile/config/branding
storage. Selecting artwork never uploads it to GitHub or another server.
Do not copy personal images into this source repository. Fit, Fill and Center
preserve aspect ratio; check the alignment and scale in each independent tab.
Apply safely saves the selection. Keep the entire profile folder when moving
this portable preview to another writable folder.

The profile folder beside this executable holds this preview's settings,
cache and account metadata. Keep the whole preview folder together. A new
extracted folder starts a separate profile; no earlier preview is modified.
Password hashes use the OS credential vault under Waters Roux's beta identity.
No installer or automatic update is included. This is not a production release.

OFFICIAL ARTWORK AND VISUAL ACCEPTANCE
The approved standalone New Orleans background and exact original logo must
be present as source files to be included. MISSING-ARTWORK.txt lists unavailable
official files. A neutral fallback is not approved New Orleans artwork.
The reference screenshot is never used as wallpaper and its sample account,
password, version, status and population data are never displayed.
Changing a personal login/loading logo does not change the official Windows
executable icon, product identity, About logo or copyright acknowledgements.
See BETA-27-ACCEPTANCE.md for the current acceptance and human test checklist.

Built on Aurora Viewer. LICENSE, NOTICE.md, dictionary and emoji licenses
are included. Source: https://github.com/WynnieOS/Waters-Roux-Viewer
The build commit and exact version are in BUILD-INFO.txt.
'@
$startHere.Replace('BETA 27', $BetaLabel.ToUpperInvariant()).Replace('Beta-27', $previewFolderLabel).Replace('BETA-27-ACCEPTANCE.md', $acceptanceLabel) |
    Set-Content -LiteralPath (Join-Path $Out 'START-HERE.txt') -Encoding utf8
$version = (& (Join-Path (Resolve-Path $Out).Path 'waters-roux-viewer.exe') --version | Out-String).Trim()
if ($LASTEXITCODE -ne 0 -or $version -notmatch '^Waters Roux Viewer ') { throw 'Packaged executable is not Waters Roux branded.' }
$commit = (git -C $source rev-parse HEAD | Out-String).Trim()
@(
    "Waters Roux Viewer — $BetaLabel"
    $version
    "Source commit: $commit"
    'Development preview only; this is NOT a public beta release.'
    'The full feature checklist, in-world QA and owner approval are required before distribution.'
) | Set-Content -LiteralPath (Join-Path $Out 'BUILD-INFO.txt') -Encoding utf8
$missingArtwork = [Collections.Generic.List[string]]::new()
foreach ($name in @('waters-roux-logo.png', 'waters-roux-background.png', 'waters-roux-loading-background.png')) {
    $original = Join-Path $source "assets/branding/$name"
    if (Test-Path -LiteralPath $original -PathType Leaf) {
        $packaged = Join-Path $Out "assets/branding/$name"
        $originalHash = (Get-FileHash $original -Algorithm SHA256).Hash
        $packagedHash = (Get-FileHash $packaged -Algorithm SHA256).Hash
        if ($originalHash -ne $packagedHash) { throw "Official artwork changed during packaging: $name" }
        "$name SHA256: $originalHash" | Add-Content -LiteralPath (Join-Path $Out 'BUILD-INFO.txt') -Encoding utf8
    } else {
        $missingArtwork.Add($name)
    }
}
if (-not (Test-Path -LiteralPath $icon -PathType Leaf)) { $missingArtwork.Add('waters-roux.ico (derived only from the approved original logo)') }
if ($missingArtwork.Count -gt 0) {
    @(
        ($BetaLabel.ToUpperInvariant() + ': OFFICIAL ARTWORK IS INCOMPLETE; VISUAL DESIGN IS NOT APPROVED.')
        'Missing source files:'
        $missingArtwork
        ''
        'No logo was fabricated. The system application icon and neutral background fallback are temporary.'
        'Personal artwork can be chosen locally in Login Appearance / Branding without rebuilding.'
    ) | Set-Content -LiteralPath (Join-Path $Out 'MISSING-ARTWORK.txt') -Encoding utf8
}
$acceptancePath = Join-Path $source $acceptanceSource
if (-not (Test-Path -LiteralPath $acceptancePath -PathType Leaf)) {
    throw "Required acceptance checklist missing: $acceptanceSource"
}
Copy-Item -LiteralPath $acceptancePath -Destination (Join-Path $Out $acceptanceLabel)
