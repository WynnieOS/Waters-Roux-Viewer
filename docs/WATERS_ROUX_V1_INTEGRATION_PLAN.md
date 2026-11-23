# Waters Roux Viewer v1.0.0 — Integration Plan

**Status:** v1.0 development branch created 2026-11-23; not yet built, integrated with Aurora 0.9.12, tested, tagged, or released.

**Owner direction:** Call the next complete Waters Roux viewer **v1.0** rather than another Beta 28 variant. Target is a privately distributed family build after acceptance. No public release or announcement without explicit owner approval.

## Protected sources and boundaries

- Waters Roux repository: `WynnieOS/Waters-Roux-Viewer`.
- Protected source baseline: `waters-roux-v3-unified-review`; leave it and `main` unchanged.
- New integration branch: `integrate/waters-roux-v1.0-aurora-0.9.12`.
- Upstream source to evaluate: `Aurora-Viewer/Aurora-Viewer`, pinned to the **v0.9.12** release tag after verifying its exact SHA. Updates are reviewed and ported/cherry-picked into Waters Roux; no unreviewed mass replacement.
- Do not create a `v1.0.0` release tag or distribute a v1.0 final installer until feature-complete, security-checked, built, and accepted.
- Do not modify Aurora's upstream repository; retain all applicable GPLv3, source, and third-party attribution requirements.

## Preserve the previously resident-approved capabilities

The earlier owner comparison audit recorded working behavior for Everyday/Studio/Creator, Name Tag Studio, four-part Login Appearance Manager, Machinima Bar, custom Build/Edit beam, residents count, PhotoTools, custom voice indicator colors, and MP4 recording. Protect these from regressions, but reconfirm on the exact v1.0 build; previous approval is not a v1.0 acceptance result.

Previously partial: simplified About Waters Roux interface with legally required credits accessible separately; three-tab Personal Lighting interface and expected behavior. Finish and validate these.

## Inherit the full outstanding acceptance scope

- Master 18-section feature and release gate: [Issue #25](https://github.com/WynnieOS/Waters-Roux-Viewer/issues/25).
- Post-V3 deficiencies, seven additions, Windows/Linux requirements: [Issue #71](https://github.com/WynnieOS/Waters-Roux-Viewer/issues/71).
- Aurora update intake policy: [Issue #34](https://github.com/WynnieOS/Waters-Roux-Viewer/issues/34).
- Current consolidated draft integration to evaluate, not blindly merge: [PR #87](https://github.com/WynnieOS/Waters-Roux-Viewer/pull/87).
- Existing acceptance source: [Beta 28 V3 acceptance document](WATERS_ROUX_BETA28_V3_ACCEPTANCE.md). These requirements continue to apply to v1.0.

## Environment/weather approach

Integrate Aurora's current **personal** weather engine and standard Second Life EEP behavior. Preserve local weather visuals. Future cross-viewer weather synchronization is a separate investigation and requires a design compatible with Linden Lab's Third-Party Viewer Policy before use on the Second Life grid. Do not restore the former hidden EEP weather-code mechanism by default.

## Build/version plan

- Public-facing product name: **Waters Roux Viewer**.
- Target stable version: **1.0.0** (not yet attained).
- While under development, use a clearly marked dev/preview identifier; do not report untested source as released v1.0.0.
- Version metadata, About window, EXE details, updater feed, package name, and installer must agree on the exact tested artifact. Current version logic is derived from Git tags via `crates/aurora-version/build.rs`; avoid tagging stable 1.0.0 until approved.
- Preserve install paths, existing settings, artwork, credentials, screenshots, and recordings; test safe migration/rollback.
- Windows x64 and Linux x86-64 builds and exact-artifact QA; macOS later, separately.
- Final resident in-world acceptance by owner/Devin before calling v1.0 complete. Never treat a draft PR or green compile alone as acceptance.

## First implementation tasks (not yet completed)

1. Record the exact Waters Roux protected baseline SHA and Aurora v0.9.12 tag SHA; review divergence and packaging/version code.
2. Inventory PR #87 and outstanding changes from issues #25/#71; make a per-feature source/test matrix.
3. Create isolated, reviewable Aurora compatibility update changes without overwriting Waters Roux-specific UI/branding/settings.
4. Complete outstanding features and partial audit items, then run Windows/Linux and in-world regression.
5. After all gates pass, produce a v1.0 candidate for family testing; only release stable v1.0.0 after owner acceptance.
