<# 
Stage an independently supplied FFmpeg executable alongside a Waters Roux
portable TEST build. Pin the HTTPS archive and its verified SHA256. Never
download tools when the resident launches the viewer.

FFmpeg is a separate GPLv3 program; preserve its license and source details.
A public distribution needs a separate licensing/source-compliance review.
#>
param(
    [Parameter(Mandatory)][string]$Out
)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

# Gyan Doshi release essentials 9.0.2, including GPL libx264 and AAC.
# https://www.gyan.dev/ffmpeg/builds/
$version = '9.0.2'
$archiveUrl = 'https://www.gyan.dev/ffmpeg/builds/packages/ffmpeg-9.0.2-essentials_build.zip'
$sha256 = '60f467265b1e312373dbcd92200c2618a74850f98d3d078e94296bb3fa2047ba'
$work = Join-Path $env:RUNNER_TEMP 'waters-roux-pinned-ffmpeg'
$zip = Join-Path $work 'ffmpeg.zip'
$expanded = Join-Path $work 'expanded'
New-Item -Path $work -ItemType Directory -Force | Out-Null
New-Item -Path $expanded -ItemType Directory -Force | Out-Null
New-Item -Path $Out -ItemType Directory -Force | Out-Null

if (-not (Test-Path -LiteralPath $zip)) {
    Invoke-WebRequest -Uri $archiveUrl -OutFile $zip -MaximumRedirection 5
}
$actual = (Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash.ToLowerInvariant()
if ($actual -ne $sha256) {
    Remove-Item -LiteralPath $zip -Force -ErrorAction SilentlyContinue
    throw "Pinned FFmpeg archive failed SHA256 validation. Expected $sha256, got $actual"
}
Expand-Archive -LiteralPath $zip -DestinationPath $expanded -Force

$ffmpeg = Get-ChildItem -LiteralPath $expanded -Recurse -File -Filter ffmpeg.exe | Select-Object -First 1
$ffprobe = Get-ChildItem -LiteralPath $expanded -Recurse -File -Filter ffprobe.exe | Select-Object -First 1
$license = Get-ChildItem -LiteralPath $expanded -Recurse -File |
    Where-Object { $_.Name -match '^LICENSE(\.txt)?$' } | Select-Object -First 1
if (-not $ffmpeg -or -not $ffprobe -or -not $license) {
    throw 'The pinned FFmpeg archive is incomplete (encoder, probe, or GPL license missing).'
}
$toolsDir = Join-Path $Out 'tools'
$licenseDir = Join-Path $Out 'third-party/ffmpeg'
New-Item -Path $toolsDir -ItemType Directory -Force | Out-Null
New-Item -Path $licenseDir -ItemType Directory -Force | Out-Null
Copy-Item -LiteralPath $ffmpeg.FullName -Destination (Join-Path $toolsDir 'ffmpeg.exe') -Force
Copy-Item -LiteralPath $ffprobe.FullName -Destination (Join-Path $toolsDir 'ffprobe.exe') -Force
Copy-Item -LiteralPath $license.FullName -Destination (Join-Path $licenseDir $license.Name) -Force

@"
Waters Roux H.264 recording uses the independently built FFmpeg $version
executable (GPLv3) as a separate child process. This build is for testing.
Original binary source: $archiveUrl
Distribution SHA256: $sha256
Source: https://github.com/FFmpeg/FFmpeg/tree/n9.0.2
Source/library and additional build information:
https://www.gyan.dev/ffmpeg/builds/
See third-party/ffmpeg/$($license.Name) for the distributed license.
This is not a representation that public distribution compliance is complete.
"@ | Set-Content -LiteralPath (Join-Path $Out 'THIRD-PARTY-FFMPEG.txt') -Encoding utf8

$encoders = & (Join-Path $toolsDir 'ffmpeg.exe') -hide_banner -encoders 2>&1 | Out-String
if ($LASTEXITCODE -ne 0 -or $encoders -notmatch 'libx264' -or $encoders -notmatch '\baac\b') {
    throw 'Packaged FFmpeg cannot encode H.264 through libx264 and AAC.'
}
& (Join-Path $toolsDir 'ffmpeg.exe') -hide_banner -version | Select-Object -First 1
if ($LASTEXITCODE -ne 0) { throw 'Packaged FFmpeg executable failed to launch' }
"Bundled and verified pinned FFmpeg $version for independent H.264/MP4 capture."
