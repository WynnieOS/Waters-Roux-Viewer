# Waters Roux Viewer v1.0.0 — Integration Plan

**Status:** v1.0 development branch created 2026-11-23; not yet built, integrated with Aurora 0.9.12, tested, tagged, or released.

**Owner direction:** Call the next complete Waters Roux viewer **v1.0** rather than another Beta 28 variant. Target is a privately distributed family build after acceptance. No public release or announcement without explicit owner approval.

## Protected sources and boundaries

- Waters Roux repository: `WynnieOS/Waters-Roux-Viewer`.
- Protected source baseline: `waters-roux-v3-unified-review`; leave it and `main` unchanged.
- New integration branch: `integrate/waters-roux-v1.0-aurora-0.9.12`.
- Upstream release reference (verified): `Aurora-Viewer/Aurora-Viewer` **v0.9.12**, commit `d6c2f58effee443e0564458ffea0c7f4c016d424`. This is an **integration target**, not a claim that all of v0.9.12 is currently integrated. Updates are reviewed and ported individually; no unreviewed mass replacement.
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

## 2026-11-23 implementation log (candidate only)

- `39c7837ea3e898433e4c3b1bcdd5905272057b20`: ported v3 PR #84 three-tab Atmosphere/Sun/Moon Personal Lighting UI. Target file's original blob matched the PR's exact preimage; no unrelated file overwritten.
- `fb71b53c92baa07d81ac98b02c08c85bd4675fd1`: ported v3 PR #85 100-frame held XYZ spinner regression test. The test is not evidence of a resident bug fix until it runs on the v1 package.
- `73c9298e662d4efe03c69edd9fb884618b8253c3`: first **Aurora v0.9.12 upstream code port**, from merged Aurora PR #3082. In `screen_rate.rs`, skip display-driver refresh-rate polling while minimized; preserve previous rate and resume after restoration. Original Waters file blob exactly matched Aurora PR's preimage. The upstream regression test is included.
- The above are surgical changes on the protected Waters V3 foundation, not a full upstream sync or a finished v1.0 viewer. Status: **integrated source, unverified on v1**.
- Full integration must still review the 0.9.12 voice, networking, RLVa, render, weather and other changes. The former shared EEP-code weather feature remains excluded from active Second Life support while a compliant sync design is researched.
- Build, packaging, Windows/Linux CI, exact-artifact security and resident/Devin testing have NOT been certified for this v1 branch as of this log.
