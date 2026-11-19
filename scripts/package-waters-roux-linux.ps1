<# 
Waters Roux Linux development packaging only.
Preserves the inherited internal updater manifest until its migration is
implemented and tested; never publishes a viewer or changes an installation.
#>
param(
    [Parameter(Mandatory)][string]$Exe,
    [Parameter(Mandatory)][string]$Out
)
$ErrorActionPreference = 'Stop'
$source = Split-Path $PSScriptRoot -Parent
if (-not (Test-Path -LiteralPath $Exe -PathType Leaf)) { throw "Linux viewer executable missing: $Exe" }
if (Test-Path -LiteralPath $Out) { throw "Choose a new output folder; cannot overwrite $Out" }

& (Join-Path $PSScriptRoot 'package.ps1') -Exe $Exe -Out $Out -Source $source
if ($LASTEXITCODE -ne 0) { throw 'Base viewer packaging failed' }

$oldBinary = Join-Path $Out 'aurora-viewer'
$newBinary = Join-Path $Out 'waters-roux-viewer'
if (-not (Test-Path -LiteralPath $oldBinary -PathType Leaf)) {
    throw 'Base package did not include the expected Linux executable'
}
Move-Item -LiteralPath $oldBinary -Destination $newBinary -ErrorAction Stop
if ($IsLinux) {
    & chmod +x $newBinary
    if ($LASTEXITCODE -ne 0) { throw 'Cannot preserve Linux executable permissions' }
}

# Do not distribute a contradictory Aurora desktop entry beside Waters Roux.
foreach ($old in @('aurora-viewer.desktop', 'aurora-viewer.svg')) {
    $oldPath = Join-Path $Out $old
    if (Test-Path -LiteralPath $oldPath) { Remove-Item -LiteralPath $oldPath -Force -ErrorAction Stop }
}
$entry = Join-Path $source 'packaging/linux/waters-roux-viewer.desktop'
$icon = Join-Path $source 'assets/branding/waters-roux-fleur-de-lis.svg'
foreach ($required in @($entry, $icon)) {
    if (-not (Test-Path -LiteralPath $required -PathType Leaf)) {
        throw "Required approved Waters Roux desktop asset missing: $required"
    }
}
Copy-Item -LiteralPath $entry -Destination (Join-Path $Out 'waters-roux-viewer.desktop') -ErrorAction Stop
Copy-Item -LiteralPath $icon -Destination (Join-Path $Out 'waters-roux-viewer.svg') -ErrorAction Stop

$desktopText = Get-Content -LiteralPath (Join-Path $Out 'waters-roux-viewer.desktop') -Raw
foreach ($line in @('Name=Waters Roux Viewer', 'Exec=waters-roux-viewer', 'Icon=waters-roux-viewer', 'StartupWMClass=waters-roux-viewer')) {
    if (-not $desktopText.Contains($line)) { throw "Invalid desktop entry: missing $line" }
}
$versionOutput = (& (Resolve-Path -LiteralPath $newBinary).Path --version | Out-String).Trim()
if ($LASTEXITCODE -ne 0 -or -not $versionOutput.StartsWith('Waters Roux Viewer ')) {
    throw "Wrong viewer identity in Linux binary: $versionOutput"
}

@(
    'WATERS ROUX VIEWER — LINUX X86-64 DEVELOPMENT PACKAGE'
    ''
    'This archive is for engineering validation, not an approved beta or public release.'
    "Compiled version: $versionOutput"
    'Build target: Linux x86-64'
    'Run ./waters-roux-viewer from the extracted folder.'
    'The waters-roux-viewer.desktop file needs an absolute Exec path when installed into a desktop menu.'
    'The inherited aurora-install.json is internal updater compatibility metadata,'
    'NOT an endorsement or evidence that automatic updates and profile migrations are complete.'
    'Do not overwrite a working viewer or existing profile.'
    'LICENSE and NOTICE.md contain required upstream and third-party notices.'
) | Set-Content -LiteralPath (Join-Path $Out 'WATERS-ROUX-LINUX-README.txt') -Encoding utf8

foreach ($required in @('waters-roux-viewer', 'waters-roux-viewer.desktop', 'waters-roux-viewer.svg', 'LICENSE', 'NOTICE.md')) {
    if (-not (Test-Path -LiteralPath (Join-Path $Out $required) -PathType Leaf)) {
        throw "Incomplete Waters Roux Linux package: $required"
    }
}
Write-Host "Branded Linux package assembled for QA: $Out"
