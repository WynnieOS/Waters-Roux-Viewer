# Waters Roux — Aurora upstream maintenance and Beta V3 workspace integration

**2026-11-16 status:** Engineering staging only. Nothing in this document approves a public viewer build.

## Technical foundation versus product design

- Keep Aurora Viewer as the tracked source foundation. This is an Aurora-derived GPLv3 viewer; preserve mandatory source attribution/licensing.
- Waters Roux owns its visible branding, glass login and wallpapers, settings, presets, video recording, screenshots, machinima controls, Name Tag Studio and workspace UX. Never overwrite these in an upstream update.
- Import critical Second Life protocol and compatibility, security, and GPU-stability fixes first, after review.
- Review PBR, OIT transparency, shadow, material, reflection, camera smoothness and performance improvements on a separate upstream-intake branch. Cherry-pick or adapt relevant changes; do not merge all of upstream `main` into a user build.
- Optional Aurora features, including weather and Aurora-specific UI gimmicks, are not required for parity.

## Candidate upstream change intake

| Candidate | Source | Decision |
|---|---|---|
| Exact per-pixel alpha sorting | Aurora PR #3618 | Evaluate for visual correctness, performance and VRAM |
| Stochastic alpha ordering | Aurora PR #3663 | Evaluate after Exact and with MSAA |
| Camera smoothing improvements | Aurora PR #2859 | Compare and preserve Waters Roux's POV stabilization |
| Security, GPU loss and crashes | Review upstream deltas since Nov 9 fork | Prioritize validated fixes |
| Weather features | Aurora PR #2410 | Optional, not a prerequisite |

Document upstream SHA/PR author and licensing for every port. Test on the same machine/scene where possible. Keep names and credits in source notices.

## Beta V3 Everyday / Studio / Creator workspace implementation

- Original feature: Waters Roux PR #3. Reintegrated on isolated V3 QA branch instead of pulling old branch history over newer branded code.
- Access after tested build: **Preferences > Layout > Waters Roux workspaces > Active workspace**.
- Everyday: chat, inventory, people and minimap.
- Studio: camera, animation/posing tools, snapshot, and Machinima Studio.
- Creator: inventory, land/area search and Quick Preferences.
- Per-mode positions saved in `workspaces/<mode>.ron`; relevant open-panel state in `workspaces/<mode>.json`. The current mode is also captured on normal viewer shutdown.
- Tests: EN/FR/JA translations; mode/default selection; clean and existing profiles; restart persistence; three-way switching; original login/toolbar/recorder/Name Tag Studio preservation; no accidental removal of unrelated panels.
- Add a toolbar mode switcher only after the base interaction has passed real Windows and grid tests.

## Blockers found during status investigation

Combined Phase 2 staging workflow `43843743095` failed **2** tests after **6,812** passed:
- `graphics_presets::saved::tests::graphics_saved_sanitizes_external_values_and_supports_older_entries` asserts the obsolete <=512m limit, despite V3's 1,024m change;
- `ui::toolbar::tests::toolbar_catalogue_holds_firestorm_commands_and_runs_the_ready_ones` fails after combining additional toolbar patches.

The guarded integration withheld a source commit; regular Phase 2 CI passing does not erase these failures. Repair the tests/behavior in the actual combined tree and rerun before any package.

### Release gates
No automatic merger of Aurora changes into `main`. No public V3 installer or v1.0 until:
1. Combined Windows format / Clippy / tests and viewer smoke tests pass.
2. Approved Waters Roux graphics, login, image/logo assets and user settings remain intact.
3. Owner and Devin test rendering, windows, SLURL, in-world recording, audio, POV motion and creator tools.
4. Exact deliverable receives security/FFmpeg/license validation and resolves Safe Browsing issues.

Source: https://github.com/WynnieOS/Waters-Roux-Viewer ; upstream: https://github.com/Aurora-Viewer/Aurora-Viewer
