# Waters Roux Viewer — Linux x86-64 Development QA

**Status: engineering preview, not an approved Beta 28 V3 or public release.**

The isolated Linux package pipeline produces a `.tar.gz` archive from
`fix/waters-roux-v3-linux-branded-package`. A green build verifies compilation,
a correct Waters Roux desktop identity, the archive checksum, and preservation of
included license notices. It **does not** establish Second Life in-world behavior,
installed desktop integration, recording/microphone permission, Linux updater
support, settings migration, or graphics stability.

## Current target
- Linux x86-64 with glibc >= 2.39 (built on Ubuntu 24.04).
- Vulkan-capable graphics and an X11/Wayland desktop.
- System sound/voice dependencies (ALSA/PulseAudio/PipeWire, as appropriate).
- No macOS release dependency.
- Windows remains a separate required platform; macOS is deferred.

## Engineering-only archive
The CI archive name follows:
`Waters-Roux-Viewer-Beta-28-V3-DEVELOPMENT-linux-x86_64.tar.gz`

Extract into a **new** folder, not over a working viewer. The binary is
`waters-roux-viewer`; Linux dock/menu integration uses
`waters-roux-viewer.desktop`, icon `waters-roux-viewer.svg`, and
Wayland app_id / X11 WM_CLASS `waters-roux-viewer`.
The original Aurora name remains in copyright acknowledgements, upstream
source implementation modules, and internal updater migration metadata when
needed for compatibility.

## Not yet passed / release blockers
- [ ] Review and test actual window/dock launcher in GNOME/KDE, X11 and Wayland.
- [ ] Confirm secondlife:// handler registration/ownership does not conflict
  with Aurora or other viewers.
- [ ] Move profile, caches and settings from old Aurora-derived paths only
  through a tested, reversible migration; never delete existing resident files.
- [ ] Independently verify Linux graphics, media/plugins, voice and microphone,
  recording and camera behavior in-world.
- [ ] Verify downloadable archive hash and library dependencies on supported
  distributions.
- [ ] Implement Waters Roux-controlled update policy, authenticity/rollback
  and install upgrades; don't mistake inherited `aurora-install.json`
  for working Waters Roux automatic updates.
- [ ] Complete the owner audit #71 and human acceptance before public v1.0.

Source of truth: #71 (post-V3 audit), #25 (master feature gate),
#34 (Aurora upstream intake). This document does not replace upstream
`docs/LINUX.md`, which currently documents Aurora's own release channels.
