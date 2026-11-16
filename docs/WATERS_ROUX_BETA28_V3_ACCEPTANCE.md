# Waters Roux Viewer — Beta 28 V3 Master Acceptance Checklist

**Status: PLANNED; NONE of the unchecked items below is certified implemented or tested.**

**Resident directive, 2026-11-16:** deliver **one more beta** named **Beta 28 V3** containing the entire laundry list. Owner and Devin will test it. Fix any resulting defects and pursue **official Waters Roux v1.0.0**. No partially fixed intermediate beta replaces the current personalized installed viewer. Keep all approved branding, settings, glass login, artwork and functioning tools.

## Meaning of checklist states

- `[ ]` = acceptance **not yet verified** (even if existing code looks implemented).
- An item may be checked only after code review, appropriate automated validation and in-viewer human acceptance if it changes viewer behavior.
- Track implementation PR/commit, Windows QA workflow run, resident testing results, and failures in follow-up notes. Never substitute a source marker or synthetic fixture for in-world verification.
- For functions impossible or unsafe under SL permissions or available renderer capabilities, document the specific blocker and request an explicit scope decision; do not quietly omit the item.

## 1. Build/Edit menu (Firestorm-familiar UX; preserve ALL settings)
- [ ] Restore intuitive Firestorm-style organization of Position, Rotation, Size, Edit Linked, Select Face and existing advanced build settings without discarding existing options.
- [ ] XYZ numeric arrows **auto-repeat on hold** and support fine/coarse adjustment without unexpected large jumps; precise typing still works.
- [ ] Test optional click-drag numeric scrubbing where supported; validate keyboard navigation, precision, scale and rotation clamps.
- [ ] Verify permissions, selection highlight/gizmos and transform behavior on prims, linked objects, mesh, attachments and different UI DPI settings.

## 2. Camera and movement control floaters
- [ ] Resize freely in both directions, including proportional layout without clipping.
- [ ] Provide a real minimize/restore action; collapsed controls do not obstruct the screen.
- [ ] Persist floater size and position; keep separate camera and movement behavior working.

## 3. Menu branding and product file identity
- [ ] Replace crown **Waters Roux menu** icon with a branded fleur-de-lis.
- [ ] Remove accidental **Aurora Viewer** product naming from screenshots/photos folder, recordings, visible export paths, window titles, shortcut/install names, logs and crash-report UI.
- [ ] Preserve/migrate existing resident files and settings; never delete user snapshots or content.
- [ ] Keep required upstream attribution, license notices and original source history intact; only remove inappropriate *product branding*.

## 4. SLURL address bar
- [ ] Paste complete Second Life SLURLs and valid `secondlife://` location references.
- [ ] Correctly decode region names, coordinates, URL encoding and Ctrl+V/Enter.
- [ ] Open map/teleport flow without bypassing RLVa, parcel constraints or simulator permissions; handle invalid URLs cleanly.

## 5. Recorder and Flycam
- [ ] Quick Record start/stop hotkey works both inside and outside Flycam.
- [ ] Flycam-status overlay supports Visible / Brief / Hidden without losing flycam indication where the user wants it.
- [ ] Produce actual H.264 MP4 rather than an AVI-only recording; support properly synchronized optional AAC/system/voice capture subject to permissions.
- [ ] Support normal/narrative camera, stabilized avatar POV, HUD/UI toggles, recording region/full-screen capture, resolutions and 30/60 FPS where supported.
- [ ] Eliminate 2x speed, camera jitter, near-static frames, premature end, stuck finalize, Windows console popups and abandoned PCM/intermediate files after successful stop.
- [ ] Decode and inspect **actual in-world video** with motion, pans, speech/system audio and >4-minute recording; duration matches stopwatch, no playback freeze. Track captured/repeated/dropped frames.

## 6. Voice indicators
- [ ] Custom persisted colors for Idle / Speaking / Loud Speaking / Muted.
- [ ] Accessible high-contrast defaults across bright/dark themes without changing actual voice function.

## 7. Viewer themes and readability
- [ ] Legible, friendly Waters Roux default, multiple palettes/themes and accent controls.
- [ ] Preserve glass look and user configuration; preview and save theme changes.
- [ ] All menus, icons, text and indicators remain readable at common UI scales; no reset of toolbar/workspaces.

## 8. Cinematic head/body tracking
- [ ] Select supported head, torso, arm and other applicable avatar skeleton targets.
- [ ] Adjustable follow sensitivity, smoothing, presets and transitions without snapping.
- [ ] Cooperate with relevant Flycam/Machinima controls; fall back gracefully where target is unavailable.

## 9. Name Tag Studio and normal name tags
- [ ] Customize both custom Studio and **regular Second Life name tags**: font, size, weight, color, opacity and background.
- [ ] Accept supported PNG/JPG/GIF backgrounds where appropriate, with safe file validation and animation handling.
- [ ] Live preview, saved presets, display name/group-title compatibility and persistence across restart.

## 10. Toolbar icons / window access
- [ ] Unique distinctive icon per major action, notably Group Titles vs Name Tag Studio.
- [ ] Distinguish Machinima, Camera Controls and PhotoTools; remove redundant PhotoTools Camera entry.
- [ ] Repair missing artwork/icons and tooltips; no greyed-out controls for features actually available.
- [ ] Preserve customizable toolbar/menu placement and label-free discoverability.

## 11. Draw distance and render stability
- [ ] Draw distance supports target **1,024 m** where renderer supports it, with lower values and safe performance scaling.
- [ ] Validate FPS, visibility, LOD and settings persistence in demanding scenes.

## 12. Firestorm-inspired PhotoTools
- [ ] Expand camera, lighting, shadows, depth-of-field, atmosphere and photography controls.
- [ ] Save/restore photographic presets.
- [ ] Integrate cleanly with existing windows without duplication or hidden controls.

## 13. Personal Lighting
- [ ] Detailed brightness, ambient, shadow, environment and applicable EEP options.
- [ ] Save presets and avoid contradictory duplicate controls.
- [ ] Respect simulator estate/environment restrictions and local-only state.

## 14. Advanced graphics and effects
- [ ] Verify supported volumetric lights/god rays, color grading, motion blur and advanced postprocessing.
- [ ] Review reflections, PBR, jewelry/metal shine/glitter and shadows within renderer capability.
- [ ] Expose user-friendly controls and sensible quality/performance presets; no shader regression.

## 15. Machinima Bar
- [ ] Complete camera smoothing, cinematic controls/presets, tracking and recording integration.
- [ ] Accessible audio/FPS/resolution/quality, camera POV and HUD/UI visibility.
- [ ] Integrated PhotoTools access, no duplicate windows/greyed-out options.

## 16. Weather, environment, login and product identity
- [ ] Evaluate compatible weather options and preserve correct SL EEP operation and user/environment permissions.
- [ ] Keep exact approved Waters Roux logo, glass login, top-right gear, customizable login/loading backgrounds and logos, supported asset types, existing Preferences layout.
- [ ] Show Residents in World / Grid Status and Last/Home/Region login selector; remove unwanted Aurora-style orange border.
- [ ] Preserve all original personalized branding and user files after package installation or upgrade.

## 17. Regression, security, packaging and release gates
- [ ] Preserve Poser with consent-based nearby animation, self Pose Stand, AO and animation tools, selection/edit beams, inventory tabs, mesh LOD/land-impact tools, Everyday/Studio/Creator layouts, Quick Preferences and custom pinwheel/toolbar preferences.
- [ ] No duplicate icon or missing-artwork warnings; responsive camera controls; no ordinary controls disabled by mistake.
- [ ] Windows CI: Rust fmt, clippy -D warnings, workspace tests and reproducible branded packaging; inspect **actual extracted output** for logo, UI and application metadata.
- [ ] Review source/third-party dependencies, FFmpeg provenance and license obligations; scan exact artifact/executables; investigate and resolve Chrome Safe Browsing DANGEROUS verdict **without bypassing protection**.
- [ ] Test standard installation/uninstallation/upgrades without losing residents' custom settings, screenshot/photo/recording files.
- [ ] Owner + Devin complete in-world Beta V3 acceptance and document results.
- [ ] Fix failed Beta V3 tests, rerun regression/security gates, prepare official v1.0.0 installer, release notes, user guide, support/update instructions and optional separately validated macOS build.

## 18. Official website, bugs, updates, feature requests and project metadata
- [ ] "Website" opens https://github.com/WynnieOS/Waters-Roux-Viewer until an actual Waters Roux site is published (no Aurora redirect).
- [ ] "Report a Bug" opens https://github.com/WynnieOS/Waters-Roux-Viewer/issues/new/choose; bug-report and crash-report browser flows go to the same Waters Roux repository.
- [ ] "Download / Updates" opens https://github.com/WynnieOS/Waters-Roux-Viewer/releases; preserve existing Waters Roux update feed and release-channel behavior.
- [ ] "Feature Requests" opens https://github.com/WynnieOS/Waters-Roux-Viewer/issues; provide a usable issue template and discoverable Help menu action.
- [ ] Help/Wiki, support and README links must not send Waters Roux residents to Aurora's wiki, download, feedback board or issue tracker.
- [ ] Fix GitHub repository Website metadata (currently Aurora on audit), product description, issue form descriptions and any active support/contact links.
- [ ] Remove Aurora GitHub API destinations and **never reuse Aurora's GitHub App client ID**; direct OAuth/reporting may only be enabled once a dedicated Waters Roux App is approved and tested. Browser reporting must remain available.
- [ ] Preserve source license notices, historical attribution and compatibility file paths; do not indiscriminately rename upstream source internals.
- [ ] Publish a reviewed Waters Roux privacy policy before official v1.0.0; an interim status notice must not masquerade as an approved privacy policy.
- [ ] Validate every final browser destination in actual Windows Beta V3; investigate broken or inaccessible GitHub Pages, Issues, release pages and privacy links.

## Current implementation notes

- Installed Beta V2 artifact identified conclusively by SHA256 at Actions run **43404203369**. See `WATERS_ROUX_BETA28_V3_SOURCE_RECOVERY.md`.
- Current GitHub `main` is **not** the resident-approved branded source; do not package it as Waters Roux.
- PR #14 includes work on H.264/MP4 but is draft, and live recorder behavior remains unverified.
- PR #15 documents Safe Browsing DANGEROUS and requires resolution; no previous artifact is automatically approved for distribution.
- The temporary `58a47df5f` build commit existed in the Windows CI worktree but not repository history.
- The staging branch and script only establish an auditable **starting point**, not a working V3 feature build.

## Transition to v1.0.0

Beta V3 = **feature-complete owner-test build**. Release v1.0.0 = **Beta V3 issues corrected and retested, with security, performance, installation, product documentation and explicit final owner approval**.
