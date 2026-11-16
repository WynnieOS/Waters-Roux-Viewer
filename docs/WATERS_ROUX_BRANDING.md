# Waters Roux Viewer — brand and glass UI specification

This fork is a distinct **Waters Roux Viewer** project built on Aurora Viewer.
It is not an official Aurora distribution. Preserve Aurora copyright, source
license and modification notices in About and in distributed sources.

## Approved references (received November 9, 2026)

- **Logo**: the user-supplied transparent Waters Roux emblem with a gilded
  fleur-de-lis, magnolia blossoms, dimensional lettering, and a
  "SECOND LIFE VIEWER" plaque. It must be incorporated **unchanged** once
  the original PNG assets are staged in the repository.
- **Login direction**: New Orleans / bayou at dusk; water, moored dock,
  lanterns, French Quarter architecture, pink flowers; central translucent
  rounded glass card. The login fields must remain accessible and must never
  persist a plaintext password.
- **Palette**: approved six hex codes, with a seventh Bayou Gold accent.
- **Window treatment**: real scene-visible frosted glass, not opaque panels
  and not a baked glass texture. The existing Aurora GPU blur is retained.

## Exact palette

| Role | Color |
| --- | --- |
| Magnolia blush | `#F7CBCA` |
| Warm stone | `#DDD3D3` |
| Cloud white | `#F1F7F7` |
| Frosted bayou mist | `#D5E5E5` |
| Powder blue | `#BDD7D8` |
| Deep slate | `#5D6B6B` |
| Bayou Gold | `#C99530` (working digital accent; confirm against supplied logo) |

A legible dark ink `#23363A` is used for text over light glass.
Interactive emphasis uses a deeper muted-rose tone to achieve contrast.

## Glass opacity behavior

The appearance preference is under
**Preferences → Appearance → Skin → Glass opacity**.

- Initial glass opacity: **172 / 255 (approximately 67%)**.
- User control: 10–100% opaque; a lower percentage shows more of the scene.
- Below 100%, the existing Aurora blur and tint must remain active.
- At 100%, the panel becomes opaque; the renderer can skip its glass blur.
- Change must immediately update existing windows without relogging.
- Preference must survive restarts via the existing `interface_colors` settings.
- **High Contrast** mode must continue to override translucent surfaces
  with the existing readable accessible palette.
- Reset Interface Colors restores the skin's initial glass opacity.

## Integration notes

The exact original logo and backdrop PNGs **are not yet committed**.
Do not substitute recreated or AI-generated copies. The reference login
mockup is a **design target**, not an already running product. In particular,
its example account name, sample online statistics, version, and grid status
must not be hardcoded into a real login view.

The functional login must continue to use Aurora's real account profiles,
authentication flow, grid selection, version calculation, and live status.

## Source of this first change

- `crates/aurora-viewer/src/theme.rs`: `Theme::waters_roux()`.
- `crates/aurora-viewer/src/ui/skin.rs`: first-run branded theme fallback.
- `crates/aurora-viewer/src/interface_colors.rs`: per-user glass opacity.
- `crates/aurora-viewer/src/ui/options/pages/appearance.rs`: opacity slider.
- `crates/aurora-viewer/src/interface_colors/tests.rs`: regression tests.

## Release acceptance gates

1. Windows build compiles and runs with existing Aurora source dependencies.
2. The palette loads on a clean first-run profile.
3. Moving the slider changes translucency live, and the chosen value survives
   a restart.
4. Text, icons, and inputs remain legible at both 10% and 100% opacity.
5. Glass blur has no unintended effect on FPS or input latency.
6. Original artwork is added unchanged and replaces all official Aurora
   branding in the distributed application.
7. The final product has independent settings, cache, packaging, version,
   updater, source URLs and Third-Party Viewer identification.

**Status:** First glass-theme coding stage only. The application is not yet a
fully rebranded or validated Waters Roux release.
