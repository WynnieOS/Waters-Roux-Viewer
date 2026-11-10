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

`Implemented` means source is present, with remaining validation stated explicitly.
`Tested` names recorded automated or application-capture evidence; it does not
imply human grid acceptance. `Blocked` identifies missing input or a required
human test. No acceptance item is assumed complete from an earlier preview.

| Requirement | Status | Evidence or remaining verification |
| --- | --- | --- |
| Exact official logo displayed | Blocked | Original downloadable PNG unavailable at implementation start |
| Official Windows/taskbar/launcher icon | Blocked | Requires approved original logo PNG and derived ICO |
| Approved New Orleans background | Blocked | Standalone background-only file unavailable |
| Orange login-card perimeter removed | Tested | Actual final 79d5b77e application captures show no amber perimeter/top strip; warning and keyboard-focus states are preserved. |
| Refined translucent panel and legible controls | Implemented | Actual login capture shows the glass card and readable controls; approved luxury composition remains blocked by missing artwork and owner review. |
| Responsive 1920×1080, 1600×900, 1366×768 and tall/narrow layout | Tested | Actual application captures at desktop sizes and 800×1100; existing minimum window width is 800. Image/logo geometry has bounded aspect-ratio tests. Human resizing and translated-label checks remain. |
| Login background selection and preview | Implemented | Independent PNG/JPEG import, preview, Fill/Fit/Center and transforms. Core import/persistence tests pass; native picker interaction still requires a human. |
| Independent login logo selection and preview | Implemented | Separate PNG import, alpha-preserving bytes, bounded aspect-preserving size/placement. Core tests pass; native picker interaction still requires a human. |
| Independent loading background | Implemented | Separate persisted slot and loading/teleport rendering path. Core tests pass; actual grid transition requires human testing. |
| Independent loading logo | Implemented | Separate persisted PNG slot and bounded placement above progress text. Core tests pass; actual grid transition requires human testing. |
| Apply / Cancel / Reset / Defaults | Tested | Atomic core and actual editor Cancel-button, reset, independent-section and failed-Apply tests pass in Windows CI. Full human picker-to-Apply/restart interaction remains required. |
| Local safe artwork storage | Tested | Core tests cover bounded decoding, managed names, atomic publication, exact-byte import and deletion of the original file. Personal artwork stays local. |
| Missing/corrupt image fallback | Tested | Core missing/corrupt/hidden tests pass; actual default application capture runs without official artwork. |
| Glass 10–100% opacity and persistence | Tested | Core round-trip and clamping tests pass. Live preview and existing in-world opacity path are implemented; human readability checks at endpoints remain. |
| Glass tint/accent/border/available blur controls | Implemented | Cached asynchronous login background blur, tint/accent and subtle glow controls. World panels retain existing translucency; world-scene blur is not introduced by this milestone. |
| High Contrast and keyboard accessibility | Tested | Actual diagnostic-profile capture confirms an opaque card and white labels in High Contrast; existing focus/warning states are retained. Human keyboard and assistive-technology checks remain. |
| Working authentication and remembered accounts | Blocked | Fresh Beta 27 human grid test required; agents do not connect to a grid or handle credentials. |
| World/avatar, inventory/search, Quick Preferences and camera | Blocked | Fresh Beta 27 human in-world regression test required; preceding owner test is not substituted. |
| Movement/flying/teleport/chat/environment/AO | Blocked | Fresh human in-world acceptance and performance comparison required. |
| Settings/profile coexistence with previous previews | Implemented | Portable marker and adjacent profile isolate settings/cache/logs; package refuses an existing output folder. Human side-by-side Windows extraction test remains. |
| Aurora copyright, licenses and derivative notices | Implemented | Existing About attribution and packaged LICENSE, NOTICE, dictionary/emoji notices retained; no endorsement claimed. |
| Everyday/Studio/Creator workspace branches preserved | Tested | Beta 27 descends from verified Beta 26; no unrelated branch merge. PR #3 Clippy fix remains separate. |
| Rust formatting / compilation / strict Clippy | Tested | Windows formatting, strict workspace/all-target Clippy and viewer/all-test compilation passed at 7f13526b. The measured-layout/cache follow-up requires its own rerun. Local full test/Clippy compilation exceeded 8 GiB memory. |
| Focused unit/regression tests | Tested | 18 Windows branding tests, 4 brand/glass tests, 11 local core/atomic-file tests and 13 i18n tests pass. Regression fixes and the new reimport-cache test require final Windows verification. |
| Actual interface screenshots | Tested | Native Rust/wgpu application captured on Xvfb/software Vulkan; not a design mockup or native-picker test. Final capture links are recorded below. |
| Windows portable package / launcher / executable names | Implemented | Fixed Beta 27 artifact/ZIP label, Waters Roux launcher/executable and isolated profile; Windows Actions verification recorded below. |
| Downloadable Beta 27 artifact | Implemented | Dedicated Windows Actions workflow is running for the final implementation commit; direct successful upload evidence will be recorded below. |
| Visual design approval | Blocked pending owner | Original artwork and owner's explicit review still required |

## Verification record

Initial screenshot source: `79d5b77e167dcf2ff90aef8124b4a14521c72c61`.
The compact-layout follow-up uses measured logo/header space so the whole
saved-account card, not just its button text, fits above the footer. Successful
reimports invalidate both image caches, allowing repaired managed files to
refresh without restarting. Additional captures and final verification are
recorded in the [current online guide](https://github.com/WynnieOS/Waters-Roux-Viewer/blob/feat/waters-roux-beta27/docs/WATERS_ROUX_BETA27.md).
[Actual application captures](https://github.com/WynnieOS/Waters-Roux-Viewer/tree/captures/beta27)
include 1920×1080, 1600×900, 1366×768, 800×1100, the Appearance editor,
and a High Contrast diagnostic view.
The diagnostic checkerboard capture is explicitly a test fixture, not official
artwork or proposed visual design. It verifies actual texture rendering and
frosted-panel alignment after importing, saving, deleting originals and
restarting the viewer in an isolated test profile.

Local results: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`
and a native `cargo build --locked -p aurora-viewer` pass (dev build uses
opt-level 0 for the viewer to fit the available memory). Existing Linux-only
dead-code warnings remain. Full local viewer test/Clippy compilation hit the
8 GiB memory limit; Windows strict checks are the target-platform authority.
A focused harness of the real branding/atomic-file modules passes **11 tests**.
`cargo test --locked -p aurora-i18n` passes **4 unit + 9 catalog tests**, including
English/French/Japanese key and variable parity.

The initial Windows run passed formatting, strict Clippy, compilation and all 18 new branding tests. Its detailed logs exposed six regression failures masked by a PowerShell runner bug. Commit `a2854407` corrects compact header sizing, hover contrast, selected-highlight expectations, inherited Quick Preferences styling and its obsolete footer assertion, and makes each test command fail the job immediately. One selected-reflection assertion still assumed a fixed inset in that follow-up; it now checks that the highlight belongs to the selected card independently of the theme corner radius. The final measured-layout/cache follow-up must be verified with:
`cargo fmt --all -- --check`, strict
`cargo clippy --locked --profile ci --workspace --all-targets -- -D warnings`,
viewer/all-target `cargo check`, and the brand/glass, branding, login,
interface-colors, credentials, theme-guard, Quick Preferences, CLI, About,
high-contrast, uninstall and news test filters.
[Corrected verification run](https://github.com/WynnieOS/Waters-Roux-Viewer/actions/runs/42760243768).

The portable build/artifact result is recorded below when uploaded. Do not substitute the
Beta 26 result for a Beta 27 check. Never call a mockup, a passing linter, or an
older successful in-world session proof that native pickers and the new
appearance transactions work.

No development branch may be merged into `main`, and no production release may
be published, without the owner's approval.
