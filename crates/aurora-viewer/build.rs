//! Build script: tells the viewer which cargo profile built it, for the
//! window title ("Aurora Viewer <version> (Dev)"): cargo only exposes `debug` /
//! `release` (PROFILE), while the profile's target directory names the
//! custom ones (`debugging`, `ci`). On Windows it also embeds the resources
//! of aurora-viewer.exe: its icon and its version block.

use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    // OUT_DIR = <target>/<profile dir>/build/<crate>-<hash>/out
    let out = std::env::var("OUT_DIR").unwrap_or_default();
    let parts: Vec<String> = Path::new(&out)
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    let dir = parts
        .iter()
        .rposition(|p| p == "build")
        .and_then(|i| i.checked_sub(1))
        .map(|i| parts[i].clone())
        .unwrap_or_default();
    println!("cargo:rustc-env=AURORA_BUILD_PROFILE={dir}");
    dictionaries();
    // Decode the checked-in Waters Roux artwork at build time. Both the
    // login logo and the Windows executable icon come from this one source.
    // Fail the build rather than shipping an unbranded or missing icon.
    let branded_icon = branding();

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        windows_resources(&branded_icon);
        // The hybrid-GPU hints of main.rs (gpu_hints) in the exe's export
        // table, where the NVIDIA and AMD drivers look for them: a Rust
        // executable exports nothing on its own
        if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
            for symbol in ["NvOptimusEnablement", "AmdPowerXpressRequestHighPerformance"] {
                println!("cargo:rustc-link-arg-bin=aurora-viewer=/EXPORT:{symbol},DATA");
            }
        }
    }
}

/// The spelling dictionaries (assets/dictionaries/<name>/<name>.aff and
/// .dic, kept as text in the repository) gzipped into OUT_DIR, where
/// spelling/dictionaries.rs embeds them: 22 MB of words take about 5 MB of
/// the exe, inflated on the spelling thread only for the languages in use.
fn dictionaries() {
    use std::io::Write as _;
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies CARGO_MANIFEST_DIR");
    let source = Path::new(&manifest).join("assets/dictionaries");
    println!("cargo:rerun-if-changed={}", source.display());
    let out = Path::new(&std::env::var("OUT_DIR").expect("Cargo supplies OUT_DIR")).join("dictionaries");
    std::fs::create_dir_all(&out).expect("the build folder is writable");
    let folders = std::fs::read_dir(&source).expect("assets/dictionaries is in the repository");
    for folder in folders.flatten().filter(|entry| entry.path().is_dir()) {
        for file in std::fs::read_dir(folder.path()).expect("a dictionary folder is readable").flatten() {
            let path = file.path();
            if !matches!(path.extension().and_then(|e| e.to_str()), Some("aff" | "dic")) {
                continue;
            }
            let bytes = std::fs::read(&path).expect("a dictionary file is readable");
            let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
            gz.write_all(&bytes).expect("gzip in memory");
            let name = format!("{}.gz", path.file_name().and_then(|n| n.to_str()).expect("an ASCII file name"));
            std::fs::write(out.join(name), gz.finish().expect("gzip in memory")).expect("the build folder is writable");
        }
    }
}

/// The window sets its icon at run time (app.rs), but the file itself needs
/// an icon resource: without one, Explorer, shortcuts and the taskbar pin
/// show the generic executable icon. The version block comes with it:
/// Properties > Details, and the name Windows gives the program (Task
/// Manager, volume mixer, firewall prompts) is its FileDescription.
///
/// The version is the one aurora-version resolved from the Git tag (its
/// `links` metadata), never CARGO_PKG_VERSION. A failure stops the build: a
/// release must not ship without its icon again.
fn windows_resources(icon: &Path) {

    let display = std::env::var("DEP_AURORA_VERSION_DISPLAY").expect("aurora-version's build script supplies the version");
    let numeric = std::env::var("DEP_AURORA_VERSION_NUMERIC").expect("aurora-version's build script supplies the version");
    // "major.minor.patch.distance" packed as four 16-bit fields
    let packed = numeric
        .split('.')
        .chain(std::iter::repeat("0"))
        .take(4)
        .fold(0u64, |acc, part| (acc << 16) | part.parse::<u64>().unwrap_or(0).min(0xFFFF));

    let mut res = winresource::WindowsResource::new();
    res.set_icon(&icon.to_string_lossy())
        .set("ProductName", "Waters Roux Viewer")
        .set("FileDescription", "Waters Roux Viewer")
        .set("OriginalFilename", "aurora-viewer.exe")
        .set("ProductVersion", &display)
        .set("FileVersion", &display)
        .set_version_info(winresource::VersionInfo::PRODUCTVERSION, packed)
        .set_version_info(winresource::VersionInfo::FILEVERSION, packed);
    // the viewer only: the test binaries of this crate need no resources
    if let Err(error) = res.compile_for("aurora-viewer") {
        panic!("Cannot embed the icon and version of aurora-viewer.exe (rc.exe of the Windows SDK): {error}");
    }
}


/// The original approved emblem is checked in as base64 text because the GitHub
/// repository's text-only file updater cannot write binary assets directly.
/// Decode exactly once during compilation. An absent or corrupted source is a
/// hard error, not a viewer that silently has missing artwork.
fn branding() -> std::path::PathBuf {
    use base64::Engine as _;
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies CARGO_MANIFEST_DIR");
    let source = Path::new(&manifest).join("../../assets/branding/waters-roux-logo.png.base64");
    println!("cargo:rerun-if-changed={}", source.display());
    let encoded = std::fs::read_to_string(&source).expect("Waters Roux artwork must be included in the checkout");
    let png = base64::engine::general_purpose::STANDARD
        .decode(encoded.trim())
        .expect("Waters Roux artwork base64 is invalid");
    assert!(png.len() >= 24 && png.starts_with(b"\x89PNG\r\n\x1a\n"), "Waters Roux artwork must be a PNG");
    let width = u32::from_be_bytes(png[16..20].try_into().expect("PNG width"));
    let height = u32::from_be_bytes(png[20..24].try_into().expect("PNG height"));
    assert_eq!((width, height), (128, 128), "Waters Roux icon dimensions must be 128x128");
    let out = Path::new(&std::env::var("OUT_DIR").expect("Cargo supplies OUT_DIR")).to_path_buf();
    std::fs::write(out.join("waters-roux-logo.png"), &png).expect("Cannot write embedded Waters Roux logo");
    // Modern Windows supports PNG-compressed images within an ICO container.
    // ICONDIR: reserved, kind=icon, count=1; ICONDIRENTRY points to PNG data.
    let mut ico = Vec::with_capacity(22 + png.len());
    ico.extend_from_slice(&[0, 0, 1, 0, 1, 0, 128, 128, 0, 0]);
    ico.extend_from_slice(&1u16.to_le_bytes());
    ico.extend_from_slice(&32u16.to_le_bytes());
    ico.extend_from_slice(&(png.len() as u32).to_le_bytes());
    ico.extend_from_slice(&22u32.to_le_bytes());
    ico.extend_from_slice(&png);
    let icon = out.join("waters-roux.ico");
    std::fs::write(&icon, ico).expect("Cannot write Waters Roux Windows icon");
    icon
}
