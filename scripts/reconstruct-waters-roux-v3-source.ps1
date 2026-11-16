# Rebuild the exact resident-tested Waters Roux Beta 28 V2 source *inputs*
# on a clean, isolated V3 staging worktree. Does NOT build or publish a viewer.
# The GitHub Actions original used temporary merge commits and a selected-file
# MP4 restore; see docs/WATERS_ROUX_BETA28_V3_SOURCE_RECOVERY.md.
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$root = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$brandedSha = '6cac6cfaf23d413f2bef4f9ca49e72a71a6bedd7'
$beamSha = '374be0c3e505a14a7479ac581dc1318873a5291f'
$editSha = 'ca3c650ebf4d02442e6bfd9f11fe7b119e39bc25'
$mp4Sha = '46b6e0b60034b3b05821e61c250068e86f2bc501'
$expectedBranch = 'feat/waters-roux-beta28-v3'

function GitChecked {
    param([string[]]$Arguments)
    & git @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw ("git " + ($Arguments -join ' ') + " failed with code " + $LASTEXITCODE)
    }
}

Push-Location -LiteralPath $root
try {
    $currentBranch = (& git branch --show-current).Trim()
    if ($currentBranch -ne $expectedBranch) {
        throw "Refusing to alter '$currentBranch': use a NEW isolated worktree on '$expectedBranch'."
    }
    $changes = @(& git status --porcelain)
    if ($LASTEXITCODE -ne 0 -or $changes.Count -gt 0) {
        throw 'The worktree is not clean. Preserve and commit/stash your own changes before reconstruction.'
    }
    GitChecked -Arguments @('merge-base', '--is-ancestor', $brandedSha, 'HEAD')
    $name = (& git config user.name)
    $email = (& git config user.email)
    if (-not $name -or -not $email) {
        throw 'Configure git user.name and git user.email in this isolated development environment before merging.'
    }

    # Fetch named refs to make the pinned commits available. Never silently
    # substitute a moved branch for the exact original source commit.
    $pinned = @(
        @{ Name = 'feat/waters-roux-beta27'; SHA = $brandedSha },
        @{ Name = 'fix/waters-roux-beta28-recording-beam'; SHA = $beamSha },
        @{ Name = 'fix/waters-roux-edit-transform-modes'; SHA = $editSha },
        @{ Name = 'feat/waters-roux-h264-mp4-recorder'; SHA = $mp4Sha }
    )
    foreach ($source in $pinned) {
        GitChecked -Arguments @('fetch', '--no-tags', 'origin', $source.Name)
        $actual = (& git rev-parse "refs/remotes/origin/$($source.Name)").Trim()
        if ($LASTEXITCODE -ne 0 -or $actual -ne $source.SHA) {
            throw "Source branch $($source.Name) moved from its pinned source; stop and audit instead of guessing."
        }
    }

    # Same source-order as the SHA256-matched V2 GitHub Actions job.
    GitChecked -Arguments @('merge', '--no-ff', '--no-edit', $beamSha)
    GitChecked -Arguments @('merge', '--no-ff', '--no-edit', $editSha)

    # Restore ONLY the selected MP4 implementation files. Never merge the
    # Aurora-based recorder branch wholesale over our branded login/app.
    $files = @(
        'crates/aurora-viewer/src/app/recording.rs',
        'crates/aurora-viewer/src/app/recording/mp4.rs',
        'crates/aurora-viewer/src/ui/machinima.rs',
        'scripts/bundle-waters-roux-ffmpeg.ps1',
        'scripts/integrate-waters-roux-mp4-ui.ps1',
        'crates/aurora-render/src/renderer.rs',
        'crates/aurora-viewer/src/build/manip.rs',
        'crates/aurora-viewer/src/world/eep.rs'
    )
    GitChecked -Arguments (@('restore', "--source=$mp4Sha", '--') + $files)

    $integrationScript = Join-Path $PSScriptRoot 'integrate-waters-roux-mp4-ui.ps1'
    if (-not (Test-Path -LiteralPath $integrationScript -PathType Leaf)) {
        throw 'Pinned MP4 integration script not restored; no safe app wiring is possible.'
    }
    & $integrationScript

    # Source smoke checks adapted from the successful V2 Windows integration.
    $required = @(
        'crates/aurora-viewer/src/branding.rs',
        'crates/aurora-viewer/src/ui/branding.rs',
        'crates/aurora-viewer/src/ui/branding_runtime.rs',
        'assets/branding/waters-roux-logo.png',
        'assets/branding/waters-roux-background.png',
        'assets/branding/waters-roux-loading-background.png',
        'assets/branding/waters-roux.ico',
        'crates/aurora-viewer/src/app/recording/mp4.rs'
    )
    foreach ($path in $required) {
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            throw "Source smoke check failed: $path is missing."
        }
    }
    $login = [IO.File]::ReadAllText((Resolve-Path 'crates/aurora-viewer/src/ui/login.rs').Path)
    $app = [IO.File]::ReadAllText((Resolve-Path 'crates/aurora-viewer/src/app.rs').Path)
    $build = [IO.File]::ReadAllText((Resolve-Path 'crates/aurora-viewer/build.rs').Path)
    if (-not $login.Contains('waters-roux-name')) { throw 'Waters Roux login heading is absent.' }
    if ($login.Contains('RichText::new("Aurora Viewer")')) { throw 'Legacy Aurora login heading is present.' }
    if (-not $app.Contains('/waters-roux-logo.png')) { throw 'Waters Roux runtime logo is absent.' }
    if (-not $app.Contains('self.recorder.stop_async()')) { throw 'MP4 asynchronous stop hook is absent.' }
    if (-not $app.Contains('self.recorder.poll_finalization()')) { throw 'MP4 finalization polling hook is absent.' }
    if (-not $build.Contains('ProductName", "Waters Roux Viewer"')) {
        throw 'Windows product name is not branded Waters Roux.'
    }

    Write-Host ''
    Write-Host 'Pinned Waters Roux V2 source ingredients reconstructed in the V3 worktree.'
    Write-Host 'NOT VERIFIED: the source has not been compiled, safety-scanned, or tested in Second Life.'
    Write-Host 'Next: review the exact git diff, compile/test, check all branding and recording in-world,'
    Write-Host 'then commit the integrated source after approval. Do NOT distribute this as a viewer.'
    & git status --short
}
finally {
    Pop-Location
}
