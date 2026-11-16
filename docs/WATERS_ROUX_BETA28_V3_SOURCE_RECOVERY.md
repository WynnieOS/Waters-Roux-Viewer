# Waters Roux Beta 28 V3 — pinned source recovery

**Status: source-recovery staging. This branch is not an integrated viewer build and must not be distributed.**

## Why this exists

The resident-approved, installed Beta 28 V2 package was created by GitHub Actions run [43404203369](https://github.com/WynnieOS/Waters-Roux-Viewer/actions/runs/43404203369) on 2026-11-13. GitHub records the exact distribution ZIP digest as:

```
SHA256 638ae1b9caec3b5e22df98f5f7433e92dc4e884e90b18fb4cbb45e06808804ff
Artifact: Waters-Roux-H264-MP4-UNVERIFIED-Windows.zip
Artifact ID: 14029112364
```

The installed viewer's **About Waters Roux** dialog reports short build commit `58a47df5f` (full local integration commit is not available in the remote repository). The run's full Windows job log shows the corresponding temporary merge `58a47df5` created when a branded source tree was merged with editing and recording fixes, before the MP4 files were restored and integrated. The run was triggered from `46b6e0b60034b3b05821e61c250068e86f2bc501`; **this trigger SHA is not the installed viewer's source tree**.

### Pinned source inputs (from the successful run's workflow and job log)

| Input | Exact SHA | Purpose |
| --- | --- | --- |
| `feat/waters-roux-beta27` | `6cac6cfaf23d413f2bef4f9ca49e72a71a6bedd7` | Waters Roux branded source, UI, login/settings and artwork |
| `fix/waters-roux-beta28-recording-beam` | `374be0c3e505a14a7479ac581dc1318873a5291f` | Recording, toolbars, creator UI, beams, etc. |
| `fix/waters-roux-edit-transform-modes` | `ca3c650ebf4d02442e6bfd9f11fe7b119e39bc25` | Edit/rotation correction |
| `feat/waters-roux-h264-mp4-recorder` | `46b6e0b60034b3b05821e61c250068e86f2bc501` | Selected H.264 MP4 backend, tooling and nonblocking app wiring |

## Reproduce the source without changing the existing viewer

1. Work only on the dedicated `feat/waters-roux-beta28-v3` branch in a **new, clean Git worktree**. Preserve all installed viewer and resident configuration files.
2. Run `scripts/reconstruct-waters-roux-v3-source.ps1` from a Windows PowerShell session at the repository root (Git and network access required). This script checks the branch/ancestry/clean tree, merges **pinned** recording/editing inputs and restores the exact selected MP4 files from the pinned source commit. It refuses to publish a ZIP.
3. The script runs the existing pinned `integrate-waters-roux-mp4-ui.ps1` for branded `app.rs` wiring and checks retained Waters Roux login, logo, customization modules, and MP4 hooks.
4. Inspect the full resulting diff (source, assets, settings and any conflicts). **Do not assume an integrated tree is correct simply because its source markers exist.**
5. Run Windows formatting, clippy, Rust workspace tests and a viewer smoke test; validate actual login, preferences, custom artwork, settings persistence, recording, editing and security. After review, commit the integrated tree to this dedicated branch and record its full SHA in the acceptance document.
6. Do not merge this staging branch to Aurora `main` or publish any user-facing viewer until every Beta V3 acceptance condition is met. Respect resident approval.

The initial GitHub Actions run used `git checkout --detach`, two `git merge --no-edit --no-ff` operations, `git restore --source` for selected MP4 files, and an integration PowerShell script. The steps above follow that known recipe using immutable SHA pins instead of moving branch heads.

## Known security and release blocks

- The exact Beta 28 V2 archive above is labeled `UNVERIFIED`, and the owner previously reported Chrome Safe Browsing's **DANGEROUS** classification. An earlier independent scan of a different artifact did not clear this later build. Do not advise bypassing Chrome or distributing unverified artifacts.
- Before any new Beta V3 installer: analyze the **exact newly built artifact**, its executable and FFmpeg provenance, reputation/signature issues and licensing; verify clean download with Safe Browsing enabled.
- A green automated fixture does not prove real avatar-POV recording, visible motion, correct clip duration, AV sync or absence of leftover PCM files. The owner and Devin perform the live-grid acceptance.
- Legal notices for Aurora and other third-party licensed components remain mandatory even while removing Aurora **user-facing product branding**.

## V3 target

The feature-complete V3 test build must satisfy every section in [WATERS_ROUX_BETA28_V3_ACCEPTANCE.md](WATERS_ROUX_BETA28_V3_ACCEPTANCE.md). After owner testing and corrections, proceed to Waters Roux Viewer v1.0.0.
