# Waters Roux — Beta 27 toolbar completion checklist

This is the implementation backlog for toolbar buttons deliberately marked unavailable in `crates/aurora-viewer/src/ui/toolbar/catalogue.rs`.

**Rule:** a toolbar button is ready only when it has a real, correct implementation. Do not change `ready: false` to `true` without implementing its handler, feedback, permissions and tests. A temporary alias to a different feature is not completion.

## Fixed in draft PR #5 (pending release verification)

- [x] `phototools` — opens the existing snapshot/photo panel.
- [x] `phototools_view` — opens the existing camera controls through the application's normal controls dispatch.

These commands require CI approval and a Windows in-world smoke test before release.

## Implemented in the current draft (pending CI and live-grid validation)

- [x] `avatar` — opens the existing Appearance floater.
- [x] `destinations` — opens the official Second Life Destination Guide in the built-in browser.
- [x] `howto` — opens the official Second Life Knowledge Base in the built-in browser.
- [x] `conversation_log` — opens the real local transcript of the selected conversation (Nearby Chat when no IM is selected).
- [x] `omnifilter` — opens Inventory's existing advanced filtering controls.
- [x] `flickr` — opens Flickr in the system browser; **does not publish photos from the viewer**.
- [x] `primfeed` — opens Primfeed in the system browser; **does not publish photos from the viewer**.
- [x] `discord` — opens the Discord web app; an official Waters Roux community invite is **not configured**.
- [x] `face_nearest_avatar` — turns our own avatar toward the closest avatar within 20 metres in the current region, with local-control and sitting checks.
- [x] `avatar_align` — opens a compass for local heading changes, increments and nearest-avatar facing.
- [x] `regiontracker` — dedicated UI showing live current session region and distinct visited regions from actual teleport history.
- [x] `wearable_favorites` — opens the existing inventory Favorites tab filtered to wearable clothing/body items and objects, supporting ordinary wear/attach.
- [x] `pose_stand` — enters the existing poser with the local avatar as the selected subject, allowing a self-pose session.

- [x] `360capture` — native six-face (90° per face) scene capture at a fixed eye, CPU equirectangular reprojection on a worker, and unique `*_360_*.png` export. Selects a GPU-safe square face resolution up to 1024 px, yielding up to 4096 × 2048. **Requires visual verification inside Second Life; do not yet release.**

## Remaining release blockers

The catalogue's 16 deliberately unavailable commands now have source handlers; none remain flagged `ready: false`. This does **not** certify that the viewer is feature complete, or that conditional disabling should be removed.

- [ ] Confirm Windows format/lint/test CI is green at the latest commit.
- [ ] Install a Windows build and check every new command with a real Second Life login.
- [ ] Verify six unique panorama directions, correct north/south/east/west and zenith/nadir, and no swapped/mirrored cubemap faces.
- [ ] Inspect the resulting 2:1 PNG for visible face seams, black faces, ghosting, missing mesh/avatars, and correct sky, water and lighting.
- [ ] Test VRAM-pressure fallback, an active existing photo, no world connection, teleport/relog, GPU reset and other failure paths.
- [ ] Verify that camera orientation and the avatar are unchanged by panorama capture and that any simulator-imposed camera/vision restrictions remain honored.
- [ ] Validate native 360 image metadata/export workflow before any public release.

## Other greyed-out controls

A grey appearance is not always an unfinished feature. Some existing commands are conditionally unavailable for legitimate reasons (e.g., the Photo Roll has no photos, RLV restrictions, disconnected voice). Investigate each disabled state; never bypass privacy, simulator permissions, or required state checks.

## Definition of done for each command

1. An actual command dispatcher action or working window exists.
2. The command is enabled only in valid states.
3. Clicking it performs the labeled feature (not an unrelated fallback).
4. Active/inactive button state reflects reality.
5. Regression tests and formatting/lint checks pass.
6. Manual Windows and Second Life in-world behavior are verified before calling it released.

## Development priorities

1. Finish CI and in-world verification for all sixteen connected commands in this draft.
2. Resolve selection outline/picking and move/rotate/scale gizmo visibility.
3. Validate genuine `360capture` output and improve speed, progress/cancel UX and panorama preview based on real in-world tests.
4. Test Region Tracker, Wearable Favorites and self Pose Stand against real in-world states and permission restrictions.
5. Upgrade external social links to actual opt-in integrations only if supported, with separate feature acceptance tests.
