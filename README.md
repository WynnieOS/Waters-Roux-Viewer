# Waters Roux Beta 27 actual application captures

Primary implementation: `d23d841d2d8d4b6b05cacae603c9d43e8ee83d6e` on
`feat/waters-roux-beta27`. `login-1366x768.png`, `appearance-editor.png`, and
both compact diagnostic captures use that exact committed native build.
The other desktop sizes, general custom-image and high-contrast captures use
`79d5b77e167dcf2ff90aef8124b4a14521c72c61`; the later changes correct compact
header sizing, hover contrast, tests, and repaired-image cache refresh.

Captured November 10, 2026 from the actual Rust/wgpu viewer running on Linux
Xvfb with software Vulkan, using existing offline demo/capture modes. No grid
login or personal credentials were used. These are not design mockups, Windows
taskbar proof, native file-picker proof, or human in-world acceptance.

The official downloadable logo PNG and standalone New Orleans background were
not available. Default screenshots deliberately show text identity and a
neutral gradient. The visual design is not approved. The amber perimeter and
decorative top strip are gone; warning and keyboard-focus colors remain.

`appearance-editor.png` shows the real Login Appearance / Branding window.
Its Login, Loading, Glass and Defaults tabs control independent settings.

Every `diagnostic-` screenshot uses ONLY synthetic checkerboard fixtures,
never official artwork or a proposed visual design. The real branding module
imports these into an isolated profile, saves and reloads the choices, and
can decode the managed copies after the original files are removed. Four
different image hashes are checked by the source-module smoke test. Live
loading/teleport on a grid still needs a human.

The compact MFA/account captures use existing offline demo account/landscape
fixtures. Their example text is not taken from the owner's mockup and is never
seeded in normal login. The actual card and complete login buttons fit above
the footer at 800×733. The High Contrast diagnostic shows an opaque card and
white labels. Human keyboard and assistive-technology testing remains required.

## Dimensions

- [login-1920x1080.png](login-1920x1080.png): 1920 × 1080
- [login-1600x900.png](login-1600x900.png): 1600 × 900
- [login-1366x768.png](login-1366x768.png): 1366 × 768
- [login-800x1100.png](login-800x1100.png): 800 × 1100
- [appearance-editor.png](appearance-editor.png): 1280 × 900
- [diagnostic-compact-mfa.png](diagnostic-compact-mfa.png): 800 × 733
- [diagnostic-compact-accounts.png](diagnostic-compact-accounts.png): 800 × 733
- [diagnostic-custom-images.png](diagnostic-custom-images.png): 1366 × 768
- [diagnostic-high-contrast.png](diagnostic-high-contrast.png): 1366 × 768
