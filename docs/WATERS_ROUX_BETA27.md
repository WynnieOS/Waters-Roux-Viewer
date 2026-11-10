# Waters Roux Viewer — Beta 27 acceptance and tester guide

**Development preview only. Not production-ready and not visually approved.**

Beta 27 continues the verified Beta 26 source on
`feat/waters-roux-beta27`. It preserves the Aurora-derived Rust, egui and
wgpu/Vulkan foundation. The preceding in-world smoke test is owner-reported
acceptance of an earlier preview, not proof that Beta 27 passes every case.
The Beta 26 visual design was explicitly rejected by the owner.

## Download and launch

Use the **Waters Roux — Windows preview** Actions run for the Beta 27 branch.
The artifact is **Waters-Roux-Viewer-Beta-27-Windows**, independent of the
Actions run counter. Its inner ZIP is
`Waters-Roux-Viewer-Beta-27-Windows.zip`; extract that ZIP into a **new writable
folder**, then double-click **Launch Waters Roux.cmd**. The executable is
`waters-roux-viewer.exe`. `BUILD-INFO.txt` records the exact source commit and
computed viewer version. This is an artifact, not a GitHub production release.

Keep the entire extracted folder together. The marker beside the executable
opts into the adjacent `profile/` folder, separating this preview's settings,
cache and logs from Aurora and previous Waters Roux previews. A new extraction
does not migrate old credentials or settings. Password hashes use the OS vault;
plaintext passwords are not stored in the artwork configuration.

## Artwork availability

| Official source asset | Availability at implementation start | Required handling |
| --- | --- | --- |
| Original Waters Roux logo PNG | **Blocked:** visible in chat, original downloadable file absent from workspace | Preserve exact bytes, composition, alpha and aspect ratio; no recreation or screenshot extraction |
| Approved standalone New Orleans background | **Blocked:** no background-only source file available | Do not use the mockup with login controls, sample account details or statistics as wallpaper |
| Approved standalone loading background | **Blocked:** no separate loading background file available | Retain the existing world/loading fallback; keep user choices independent |
| Windows icon derived from the original logo | **Blocked by original PNG** | Generate only from the approved logo; no substitute logo |

The source files are expected under `assets/branding/` as
`waters-roux-logo.png`, `waters-roux-background.png`, optional independent
`waters-roux-loading-background.png`, and the derived `waters-roux.ico`.
The official login and loading logos both use the exact original logo source;
custom logo selections are independent. The package includes every available file and records image
SHA256 values in `BUILD-INFO.txt`. `MISSING-ARTWORK.txt` identifies absent files.
A neutral fallback is not the approved New Orleans artwork. Neither the missing
logo nor missing background is a reason to fabricate one or claim approval.
The user's previous permission to publish the exact logo does not authorize
publication of personal images chosen later inside the application.

## Test the customization controls

1. Before authentication, open the login gear button to open **Login Appearance /
   Branding**. In-world, open **Preferences → Waters Roux → Appearance & Branding**.
2. In **Login**, choose a background PNG or JPEG. Confirm the native file picker
   opens, choose a file, inspect the preview and current-selection label, then
   try **Fill**, **Fit**, **Center**, alignment and scaling. Check that the image
   keeps its aspect ratio and that cropping is intentional.
3. Choose a transparent PNG for the login logo. Adjust its size and placement.
   Verify its alpha, complete composition and aspect ratio. The choice must not
   change the Windows executable identity or official application icon.
4. In **Loading**, choose a different background and a different transparent
   PNG logo. Change their fit, size and placement. Verify that the Login choices
   stay unchanged. Preview first; inspect an actual loading/teleport transition
   when a human logs in and teleports.
5. Check the unsaved-change indication. **Apply** saves the pending appearance;
   **Cancel** discards it. Reopen the editor after each action. Close and restart
   the viewer after Apply and confirm every independent selection remains.
6. Use the section reset and **Defaults** to restore one part or the entire
   official preset. When official files are unavailable, the application must
   remain usable and explain its fallback; a reset cannot supply missing art.
7. In **Glass**, test 10%, the default value and 100% opacity. Check the immediate
   preview, tint/accent and supported border/blur controls. Decorative lighting
   must not obscure labels. Confirm High Contrast still gives readable opaque
   surfaces and meaningful keyboard-focus and warning indicators.
8. Choose a corrupt file, an unsupported type and an unreasonable image size.
   Confirm an understandable error, no lost saved configuration and no frozen
   application. After an accepted import, move or delete the original external
   file and reopen the viewer: its managed local copy should still work.

Personal artwork belongs in Waters Roux-specific local configuration storage
under `profile/config/branding` in this portable build. The viewer manages the
imported files; users do not manually copy them into the installation or edit
source code. Artwork selection does not upload image data to GitHub or another
server. **Apply** publishes saved configuration safely; **Cancel** must not
replace the last applied configuration. Native picker verification requires an
actual picker interaction: a synthetic UI screenshot alone is insufficient.

## Acceptance matrix

`Pending` means implementation or verification is in progress. Only a recorded
result may change a row to `Implemented`, `Tested` or `Blocked`; no row is
implicitly complete because an earlier preview passed. A tested row must name
whether the evidence is a unit test, actual application capture, Windows CI or
human in-world check.

| Requirement | Status | Evidence or remaining verification |
| --- | --- | --- |
| Exact official logo displayed | Blocked | Original downloadable PNG unavailable at implementation start |
| Official Windows/taskbar/launcher icon | Blocked | Requires approved original logo PNG and derived ICO |
| Approved New Orleans background | Blocked | Standalone background-only file unavailable |
| Orange login-card perimeter removed | Pending | Inspect `glass_card()` change and actual Beta 27 capture; preserve warning/focus outlines |
| Refined translucent panel and legible controls | Pending | Inspect actual first-account and remembered-account views, including high contrast |
| Responsive 1920×1080, 1600×900, 1366×768 and tall/narrow layout | Pending | Runtime captures and resizing; logo/card/gear must remain reachable |
| Login background selection and preview | Pending | PNG/JPEG validation, real native picker, fit/fill/center, safe crop/position/scale |
| Independent login logo selection and preview | Pending | Transparent PNG, complete aspect-preserved logo, size/position; official exe identity unchanged |
| Independent loading background | Pending | Separate persisted choice, transition rendering and progress/operational text preserved |
| Independent loading logo | Pending | Separate persisted transparent PNG, size/position; login choice unchanged |
| Apply / Cancel / Reset / Defaults | Pending | Transaction unit tests plus actual UI interaction and restart verification |
| Local safe artwork storage | Pending | Bounded decoding, atomic writes, no arbitrary path reuse, no artwork transmission |
| Missing/corrupt image fallback | Pending | Validation/fallback tests and actual application recovery |
| Glass 10–100% opacity and persistence | Pending | Unit tests, immediate preview, restart and readability at both endpoints |
| Glass tint/accent/border/available blur controls | Pending | Label supported effects accurately; avoid per-frame decoding or unnecessary GPU work |
| High Contrast and keyboard accessibility | Pending | Tab/Shift+Tab, focus indication, warnings and opaque fallback |
| Working authentication and remembered accounts | Pending human test | No automated grid login or credential handling; owner must test Beta 27 |
| World/avatar, inventory/search, Quick Preferences and camera | Pending human test | Retain earlier behavior; smoke-test in the Beta 27 binary |
| Movement/flying/teleport/chat/environment/AO | Pending human test | Human in-world acceptance and performance comparison |
| Settings/profile coexistence with previous previews | Pending | Portable isolation inspection and restart test in a new extraction folder |
| Aurora copyright, licenses and derivative notices | Pending | Packaging inventory and About inspection; no endorsement claim |
| Everyday/Studio/Creator workspace branches preserved | Pending review | No unrelated branch merge; PR #3 remains separate |
| Rust formatting / compilation / strict Clippy | Pending | Record current revision's commands and results after verification |
| Focused unit/regression tests | Pending | Four choices, validation, transactions/defaults/persistence/fallback/layout/glass |
| Actual interface screenshots | Pending | Capture native application, identify source commit, dimensions and missing-art limitations |
| Windows portable package / launcher / executable names | Pending Windows Actions | Fixed Beta 27 artifact label; package same working dependencies and notices |
| Downloadable Beta 27 artifact | Pending Windows Actions | Record direct run/artifact link after successful upload |
| Visual design approval | Blocked pending owner | Original artwork and owner's explicit review still required |

## Verification record

The implementation owner will record the final commands, counts, failures,
actual screenshots and Actions links here when available. Do not substitute the
Beta 26 result for a Beta 27 check. Never call a mockup, a passing linter, or an
older successful in-world session proof that native pickers and the new
appearance transactions work.

No development branch may be merged into `main`, and no production release may
be published, without the owner's approval.
