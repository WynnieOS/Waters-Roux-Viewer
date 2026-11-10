# Waters Roux Viewer — Beta 27 acceptance and tester guide

**Development preview only. Not production-ready and not visually approved.**

Beta 27 continues the verified Beta 26 source on
`feat/waters-roux-beta27`. It preserves the Aurora-derived Rust, egui and
wgpu/Vulkan foundation. The preceding in-world smoke test is owner-reported
acceptance of an earlier preview, not proof that Beta 27 passes every case.
The Beta 26 visual design was explicitly rejected by the owner.

## Download and launch

Download the [Beta 27 Windows artifact](https://github.com/WynnieOS/Waters-Roux-Viewer/actions/runs/42762694903/artifacts/13738664714)
from the [successful Windows preview run](https://github.com/WynnieOS/Waters-Roux-Viewer/actions/runs/42762694903).
It was built from `d23d841d2d8d4b6b05cacae603c9d43e8ee83d6e` and expires
November 24, 2026. GitHub sign-in may be needed for artifact downloads.
The artifact is **Waters-Roux-Viewer-Beta-27-Windows**, independent of the
Actions run counter. Extract the GitHub download first. Its inner ZIP is
`Waters-Roux-Viewer-Beta-27-Windows.zip`; extract that ZIP into a **new writable
folder**, then double-click **Launch Waters Roux.cmd**. The executable is
`waters-roux-viewer.exe`. `BUILD-INFO.txt` records the exact source commit and
computed viewer version. This is an artifact, not a GitHub production release.

Keep the entire extracted folder together. The marker beside the executable
opts into the adjacent `profile/` folder, separating this preview's settings,
cache and logs from Aurora and previous Waters Roux previews. A new extraction
does not migrate old credentials or settings. The existing optional media-support
consent prompt may appear on first launch; choose according to your preference
before opening the login gear. Password hashes use the OS vault;
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
| Normal application / launcher / About identity | Implemented | Waters Roux window title, product text and portable launcher/executable names. Official icon is blocked below; the inherited crash-report title remains unfinished polish. |
| Official Windows/taskbar/launcher icon | Blocked | Requires approved original logo PNG and derived ICO |
| Approved New Orleans background | Blocked | Standalone background-only file unavailable |
| Orange login-card perimeter removed | Tested | Actual d23d841d login/editor/compact captures and earlier desktop captures show no amber perimeter/top strip; warning and keyboard-focus states are preserved. |
| Refined translucent panel and legible controls | Implemented | Actual login capture shows the glass card and readable controls; approved luxury composition remains blocked by missing artwork and owner review. |
| Responsive 1920×1080, 1600×900, 1366×768 and tall/narrow layout | Tested | Actual desktop and 800×1100 captures, plus committed d23d841d compact MFA/account captures at 800×733. Forty login tests pass, including complete-card visibility. Existing minimum width is 800; human resizing and translated labels remain to check. |
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
| Rust formatting / compilation / strict Clippy | Tested | Windows formatting, strict workspace/all-target Clippy and viewer/all-test compilation pass at d23d841d. Local native build passes; local full test/Clippy compilation exceeded 8 GiB memory. |
| Focused unit/regression tests | Tested | 151 passing executions across selected Windows filters, including 19 branding and 40 login tests; 1 OS credential-store test intentionally ignored. Also 11 local core/atomic-file and 13 i18n tests pass. Counts and run below. |
| Actual interface screenshots | Tested | Native Rust/wgpu application captured on Xvfb/software Vulkan; not a design mockup or native-picker test. Final capture links are recorded below. |
| Windows portable package / launcher / executable names | Tested | Windows release compile, Waters Roux version check, packaging and upload pass for d23d841d. Launcher, executable and portable marker are generated by the checked packaging script. Human Windows launch still required. |
| Downloadable Beta 27 artifact | Tested | Artifact 13738664714 uploaded successfully from d23d841d; direct link above, retention until November 24, 2026. |
| Visual design approval | Blocked | Original artwork and owner's explicit review still required |

## Verification record

Tested implementation and primary screenshot source:
`d23d841d2d8d4b6b05cacae603c9d43e8ee83d6e`.
[Actual application captures](https://github.com/WynnieOS/Waters-Roux-Viewer/tree/captures/beta27)
include the 1366×768 login, Appearance editor and 800×733 MFA/account forms from
that exact committed build. Earlier unchanged default views at 1920×1080,
1600×900, 800×1100 and the High Contrast diagnostic identify their source
revision in the capture README. The measured logo/header space keeps complete
cards above the footer. Reimporting a repaired image invalidates both caches.

The checkerboards and account examples are explicitly isolated offline test
fixtures, not official artwork, real account details or proposed visual design.
The rendering smoke test imports four distinct hashes, saves/reloads all four
selections, deletes the original files and verifies the managed images still
load. Native-picker and actual grid-transition interactions remain human tests.

Local results: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`
and a native `cargo build --locked -p aurora-viewer` pass (dev build uses
opt-level 0 for the viewer to fit the available memory). Existing Linux-only
dead-code warnings remain. Full local viewer test/Clippy compilation hit the
8 GiB memory limit; Windows strict checks are the target-platform authority.
A focused harness of the real branding/atomic-file modules passes **11 tests**.
`cargo test --locked -p aurora-i18n` passes **4 unit + 9 catalog tests**, including
English/French/Japanese key and variable parity.

Windows verification at the tested implementation revision is complete:
[successful run](https://github.com/WynnieOS/Waters-Roux-Viewer/actions/runs/42762695060).
Formatting, strict `cargo clippy --locked --profile ci --workspace --all-targets -- -D warnings`
and `cargo check --locked --profile ci --all-targets -p aurora-viewer` pass.
The detailed log contains **zero failed tests**. The runner now exits on the
first failed filter, fixing the earlier false-green PowerShell behavior.

| Windows test filter | Passed | Ignored |
| --- | ---: | ---: |
| `waters_roux_` | 4 | 0 |
| `branding` | 19 | 0 |
| `ui::login` | 40 | 0 |
| `ui::loading` | 2 | 0 |
| `login_accounts` | 6 | 0 |
| `interface_colors` | 13 | 0 |
| `credentials` | 9 | 1 |
| `ui::theme_guard` | 5 | 0 |
| `ui::quick_prefs` | 9 | 0 |
| `cli::tests` | 8 | 0 |
| `ui::about` | 4 | 0 |
| `high_contrast` | 6 | 0 |
| `uninstall` | 14 | 0 |
| `ui::news` | 12 | 0 |

These are **151 passing executions across selected filters**, not the full
viewer test suite or a human in-world result. The intentionally ignored
`credentials::tests::store_round_trip` needs an OS credential-store human test.

Remaining polish: the crash-report helper still uses the inherited Aurora
title strings. Normal login/window/About identity is Waters Roux; the exact
logo, derived icon and approved scene remain blocked by missing source files.
In-world panels keep existing translucency; world-scene blur is unfinished.

Windows delivery is complete: [successful portable build](https://github.com/WynnieOS/Waters-Roux-Viewer/actions/runs/42762694903),
[artifact 13738664714](https://github.com/WynnieOS/Waters-Roux-Viewer/actions/runs/42762694903/artifacts/13738664714),
76,392,789 bytes. The optimized release build uses the existing
`--release --locked -p aurora-viewer --features xr-static-loader` command.
Packaging verifies the Waters Roux executable identity and includes available
assets, licenses, launcher, isolated-profile marker, instructions and the
missing-art manifest. No original official logo/background/ICO file was
available to include. The Windows executable was exercised with `--version`
during packaging; interactive Windows login and native picker remain human tests.

The code in this artifact is the tested implementation commit above. A later
documentation-only commit records these results without changing application
source or rebuilding the same code. Do not substitute the
Beta 26 result for a Beta 27 check. Never call a mockup, a passing linter, or an
older successful in-world session proof that native pickers and the new
appearance transactions work.

No development branch may be merged into `main`, and no production release may
be published, without the owner's approval.
