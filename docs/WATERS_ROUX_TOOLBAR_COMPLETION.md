# Waters Roux — Beta 27 toolbar completion checklist

This is the implementation backlog for toolbar buttons deliberately marked unavailable in `crates/aurora-viewer/src/ui/toolbar/catalogue.rs`.

**Rule:** a toolbar button is ready only when it has a real, correct implementation. Do not change `ready: false` to `true` without implementing its handler, feedback, permissions and tests. A temporary alias to a different feature is not completion.

## Fixed in draft PR #5 (pending release verification)

- [x] `phototools` — opens the existing snapshot/photo panel.
- [x] `phototools_view` — opens the existing camera controls through the application's normal controls dispatch.

Both need CI approval and a Windows in-world smoke test before release.

## Still to implement

| ID | Intended behavior / investigation | Status |
| --- | --- | --- |
| `avatar` | Identify intended Avatar panel; implement correct UI/action | Missing |
| `destinations` | Open a real destination-discovery workflow | Missing |
| `howto` | Open usable viewer help/onboarding | Missing |
| `flickr` | Implement supported authenticated photo publishing, if API and permissions permit | External integration |
| `primfeed` | Verify service/API still exists and is appropriate before building | External integration |
| `pose_stand` | Implement a pose stand with self-avatar pose controls | Missing |
| `conversation_log` | Open stored conversation history with privacy safeguards | Missing |
| `regiontracker` | Implement region tracking UI and behavior | Missing |
| `wearable_favorites` | Implement wearable favorites management | Missing |
| `discord` | Configure an official community target instead of using upstream Aurora's link | Branding dependency |
| `360capture` | Render and export a genuine panoramic screenshot | Rendering feature |
| `omnifilter` | Implement the intended unified filtering UI | Missing |
| `avatar_align` | Implement safe avatar alignment controls with consent where appropriate | Avatar feature |
| `face_nearest_avatar` | Implement safe facing controls with appropriate restrictions | Avatar feature |

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

1. Finish Photo Tools PR #5 and confirm real camera/photography behavior.
2. Resolve selection outline/picking and move/rotate/scale gizmo visibility.
3. Complete locally implementable creator commands (`pose_stand`, `360capture`, `avatar_align`, `face_nearest_avatar`).
4. Complete viewer UI commands and services (`avatar`, `destinations`, `howto`, `conversation_log`, `regiontracker`, `wearable_favorites`, `omnifilter`).
5. Implement external integrations (`flickr`, `primfeed`, `discord`) only with confirmed API/service availability, branding and credentials.
