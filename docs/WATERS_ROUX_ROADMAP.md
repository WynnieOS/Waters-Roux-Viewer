# Waters Roux Viewer — build plan and acceptance gates

**Status:** under active development; not yet a complete or validated viewer.
**Base:** Aurora Viewer Rust/wgpu/Vulkan source. Firestorm and Black Dragon
are feature references, not codebases to merge wholesale.
**Distribution:** limited to owner, family and trusted friends. The GitHub
repository is currently publicly accessible; do not commit credentials,
personal account information, region addresses, or private assets.

## Release stages

### R0 — Independent foundation (in progress: PR #1)
- [x] Source fork and a separate development branch
- [x] Waters Roux six-colour palette plus working Bayou Gold accent
- [x] User-adjustable glass opacity using existing real-world blur support
- [x] Separate default configuration/cache/media storage directories
- [x] Disable Aurora automatic update checks for new profiles
- [x] Point the updater's release discovery to this fork (no Waters Roux release yet)
- [x] Waters Roux window title
- [x] Add isolated Windows check and Windows preview workflows
- [ ] GitHub Actions green on the fork
- [ ] First local preview launches
- [ ] Test opacity, high contrast, authentication UI, restore/persist
- [ ] Incorporate the exact supplied emblem and approved standalone bayou background
- [ ] Finish independent package executable name, installers, links, trademarks, release ID and UI text
- [ ] First human-validated in-grid smoke test

### R1 — Everyday workspaces
- [ ] Everyday / Studio / Creator mode switcher
- [ ] Named workspace presets built on egui layout persistence
- [ ] Customizable Quick Preferences rows
- [ ] Firestorm-style tabs linked to Aurora's saved inventory smart searches
- [ ] Audited inventory shortcuts and bulk workflows
- [ ] AO set management enhancements (set cloning already present)
- [ ] User-friendly choice of launch / startup workspace

### R2 — Creator
- [ ] LOD and per-material geometry diagnostics on existing mesh inspector
- [ ] Compare calculated weights to authoritative upload quotes
- [ ] Land impact inspector built on existing server cost queries
- [ ] Creator preset for Blender LiveMesh and build panels
- [ ] Script editing extras (snippets, compile diagnostics and convenience shortcuts)

### R3 — Studio
- [ ] Unified machinima studio and camera preset workspace
- [ ] Expanded .cube LUT and color-grading preset management
- [ ] Animation authoring project workflow improvements and exporter tests
- [ ] Extend the existing explicit consent-based local poser
- [ ] Validate multi-account/recipient consent and withdrawal with two human-operated sessions

### R4 — Advanced renderer
- [ ] Motion blur using existing temporal motion vectors; guard camera cuts
- [ ] Volumetric lighting using current sun-shadow/depth information
- [ ] Profile quality levels and measure cost separately
- [ ] Evaluate alternative anti-aliasing against existing SMAA/TAA/FSR
- [ ] Avoid blur artifacts in UI, HUD and avatar photography workflows

### R5 — Release quality
- [ ] Build, smoke and regression tests on supported target computers
- [ ] Check first-run settings isolation from upstream Aurora/Firestorm
- [ ] Independent updater feed and rollback protection
- [ ] Privacy, accessibility, performance and permission audits
- [ ] Licensing, acknowledgements, source availability and TPV policy review
- [ ] User approves a tested build before family/friend distribution

## Implementation rules

- Every requested item gets a measurable acceptance test.
- New post-processing passes must be individually disableable.
- Existing Aurora features should be improved, not duplicated.
- No runtime fake user counts, static grid status or mock login data.
- Never connect to a real SL grid using a developer's automation account.
- Preserve upstream attribution and source requirements; the Waters Roux
  name and assets are separate and not presented as Aurora-endorsed.
- No promised delivery dates until the verification and deployment gates
  have been completed and evaluated.

## Current verification

The initial branding/glass module edits are committed. Workflows have been
created to compile and test the source on Windows; **passing results are not
yet recorded** in this document. The source is not yet a released binary.

Development branches:
- `feature/waters-roux-glass-foundation`
- PR: <https://github.com/WynnieOS/Waters-Roux-Viewer/pull/1>

## Owner-tested preview and current beta — November 9, 2026

The owner reports successful Windows login to Second Life, avatar/world
rendering, inventory, camera, movement, communications, Quick Preferences
and initial glass preferences in the preceding preview. This supersedes the
initial "not tested" status above for that preview only; the new branded beta
requires fresh human validation.

The beta branch integrates the existing glass and Quick Preferences work,
adds the branded first-account glass layout and isolated portable package,
and preserves all upstream license notices. PR #3's test-only constant warning
is fixed separately on its existing branch; its workspaces are not silently
merged into the beta. Original downloadable logo/background files remain
artwork blockers until supplied. Build results will be recorded after checks.

## Beta 27 continuation

Continues Beta 26 commit 1264a7c4 on feat/waters-roux-beta27. Local-only four-slot artwork imports, transactional editor, independent loading transitions and soft glass edges are implemented. See WATERS_ROUX_BETA27.md for per-item validation status. Official artwork remains blocked on original downloadable assets. No workspace branches merged; no main merge or production release authorized.
