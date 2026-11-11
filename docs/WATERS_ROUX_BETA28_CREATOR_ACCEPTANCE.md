# Waters Roux Beta 28: Creator Features — in-world acceptance gates

**Source**: Resident testing of the actual Integrated Beta 27 Windows artifact, 10 November 2026.
**Status**: OPEN. This document tracks observed behavior and deliberately does **not** declare any missing feature implemented.

## Working: preserve, do not regress

- Poser: self posing works, and nearby-avatar pose invitations/permissions work in-world. Preserve explicit consent, revoke/stop, distance, and ownership safeguards.
- Snapshot, camera controls, existing panorama implementation remain separately accessible.

## Release-blocking gaps and explicit acceptance tests

### P0 — Interaction selection beam / edit highlighting
Observed: user can identify or interact with objects, but the familiar selection particles **from the avatar's hand to the target** are not visible. This is *not* the object particle emitter and is *not* equivalent to a selection bounding outline or manipulator.
- Show a configurable, clearly visible selection beam when the local avatar selects/edits a target, anchored to the animated hand joint, ending at the actual selected world-space hit/object position.
- The beam visually follows the hand and object as either moves, respects world occlusion and selected object state, and never appears when the tool is inactive, in inappropriate UI context, or when disabled.
- Expose beam on/off, density, color and accessibility/reduced effects. In mouselook/first-person, choose a deliberate non-obstructive presentation.
- Verify in-world on mesh and linked objects with shadows/LOD/graphics presets; screen capture proof.

### P0 — PhotoTools and PhotoTools Camera are not aliases
Code observed: `ui/bars.rs` maps `"snapshot" | "phototools"` to `panels.snapshot`, and `"view" | "phototools_view"` to generic camera controls.
- Build dedicated PhotoTools panel with working camera DoF, exposure/HDR, EEP/WindLight environment, lighting, draw distance and photo presets where supported.
- PhotoTools Camera must provide *photography-specific* camera functionality and not merely open ordinary Controls.
- Snapshot stays distinct; preserving existing photo capture.
- Add click-routing tests that assert distinct relevant panels/actions and in-world screenshots.

### P0 — Dedicated machinima bar
- Independent dockable and draggable bar, not simply a camera floater alias; remember placement and visibility; works in hidden-HUD/full-screen recording.
- Functional camera movement, smoothing, presets, keyframes or paths only when genuinely implemented; explicit indicator and controls for actual recorder.
- Test mouse/keyboard focus, UI capture exclusion and switching worlds.

### P0 — Native recorder, not just an enabled button
- Genuine screen capture pipeline from the renderer; choose file path, codec/container, framerate and resolution; Start/Pause/Resume/Stop; show elapsed time and recording status, verify playable file with audio/frames as requested.
- Stop and flush cleanly on logout, crash/restart, loss of GPU context or disk capacity; display failures instead of silent success.
- Respect explicit resident privacy, consent for voice capture, and never silently record.
- In-world acceptance: play recorded clip in independent player, compare duration, image and audio.

### P0 — Avatar-eye POV recording
- Separate first-person mode using avatar view transform that does not inherit an unrelated external camera.
- Follow avatar rotation/animation deterministically, document how avatar-head clipping is avoided, and make POV recording work with the native recorder.
- Verify first-person perspective, audio, body motion, mouselook transitions and independent stop.

### P0 — Fully customizable nametags
- Existing world name tags/nameplates and visibility settings are **not** a custom design editor.
- UI for per-user display settings: font family/size/weight, text outline/readability, background shape/opacity/image, optional animated GIF (including safe resource limits and transparent alpha), and preview before save.
- Ensure names cannot be spoofed through decorative assets; expose username/identity label and privacy controls. Persist and load local designs.
- Verify at different distances, crowds, graphics settings and Windows DPI scaling.

### P1 — Pose Stand must not be the same user experience as Poser
Observed and code confirmed: `BarAction::PoseStand` in `app.rs` checks local avatar standing, then calls `ik_editor.open_poser()`, selects self, and opens the same `"poser"` floater. The self-only condition does *not* make a distinct pose stand.
- Pose Stand: distinct avatar-only stance session with independent controls, save/load poses and explicit start/stop without launching the full multi-avatar Poser interface.
- Poser: retain full current self and consented nearby-avatar posing and invitation behavior.
- Test independent UI and state, no duplicate floating windows, consent remains intact.

## Engineering references
- `crates/aurora-viewer/src/ui/toolbar/catalogue.rs` command catalogue
- `crates/aurora-viewer/src/ui/bars.rs` toolbar dispatch, active state and tests
- `crates/aurora-viewer/src/app.rs` PoseStand/Poser dispatch, renderer and edit overlay integration
- `crates/aurora-viewer/src/build/mod.rs` selection/manipulator state; this is not itself a hand-origin selection beam
- `crates/aurora-viewer/src/ui/hud.rs`, `ui/nameplate.rs` and `settings.rs` current name-tag implementation
- `crates/aurora-viewer/src/ui/snapshot.rs` existing snapshot UI / photo settings, which should remain distinct

## Definition of done
For each checkbox-like feature above: source implementation exists, Windows compile/lint/tests pass, automated UI tests prove non-duplicate command semantics where applicable, the actual integrated artifact is generated and hash-verified, and a resident confirms it works **in Second Life**. Do not market any of the above as present in Beta 27 or a future download until separately verified.
