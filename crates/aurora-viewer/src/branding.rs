//! Local-only login/loading artwork and transactional appearance preferences.
//!
//! Personal image bytes stay in this viewer's configuration directory. Import
//! validates before decoding, preserves the exact source bytes, and never
//! changes official assets, account settings or application icon resources.
use egui::{Pos2, Rect, Vec2};
use image::{DynamicImage, ImageFormat, ImageReader, Limits};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

pub const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
pub const MAX_SIDE: u32 = 8192;
pub const MAX_PIXELS: u64 = 16 * 1024 * 1024;
const MAX_CONFIG_BYTES: u64 = 64 * 1024;
const CONFIG: &str = "appearance.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Slot {
    LoginBackground,
    LoginLogo,
    LoadingBackground,
    LoadingLogo,
}

impl Slot {
    pub fn is_logo(self) -> bool {
        matches!(self, Self::LoginLogo | Self::LoadingLogo)
    }

    fn official_file(self) -> &'static str {
        match self {
            Self::LoginBackground => "waters-roux-background.png",
            Self::LoadingBackground => "waters-roux-loading-background.png",
            Self::LoginLogo | Self::LoadingLogo => "waters-roux-logo.png",
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Fit {
    #[default]
    Fill,
    #[serde(rename = "fit")]
    Contain,
    Center,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageChoice {
    #[default]
    Official,
    Custom(String),
    Hidden,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ImageSettings {
    pub choice: ImageChoice,
    pub fit: Fit,
    pub scale: f32,
    pub x: f32,
    pub y: f32,
    /// Logo width as a fraction of the available screen/card width. The
    /// renderer always derives height from the original image aspect ratio.
    pub size: f32,
}

impl Default for ImageSettings {
    fn default() -> Self {
        Self {
            choice: ImageChoice::Official,
            fit: Fit::Fill,
            scale: 1.0,
            x: 0.5,
            y: 0.5,
            size: 0.32,
        }
    }
}

impl ImageSettings {
    pub fn normalize(&mut self) {
        self.scale = finite(self.scale, 1.0).clamp(0.25, 4.0);
        self.x = finite(self.x, 0.5).clamp(0.0, 1.0);
        self.y = finite(self.y, 0.5).clamp(0.0, 1.0);
        self.size = finite(self.size, 0.32).clamp(0.08, 1.0);
        if matches!(&self.choice, ImageChoice::Custom(name) if !safe_name(name)) {
            self.choice = ImageChoice::Official;
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tint {
    #[default]
    Mist,
    Blush,
    Cloud,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Accent {
    #[default]
    Blush,
    Blue,
    Gold,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Glass {
    pub opacity: u8,
    /// Blur of the login/loading image in image pixels, cached on workers;
    /// it does not change the renderer's existing world blur pass.
    pub blur: f32,
    pub tint: Tint,
    pub accent: Accent,
    pub glow: f32,
}

impl Default for Glass {
    fn default() -> Self {
        Self {
            opacity: 67,
            blur: 10.0,
            tint: Tint::Mist,
            accent: Accent::Blush,
            glow: 0.35,
        }
    }
}

impl Glass {
    pub fn normalize(&mut self) {
        self.opacity = self.opacity.clamp(10, 100);
        self.blur = finite(self.blur, 10.0).clamp(0.0, 24.0);
        self.glow = finite(self.glow, 0.35).clamp(0.0, 1.0);
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScreenAppearance {
    pub background: ImageSettings,
    pub logo: ImageSettings,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Appearance {
    pub login: ScreenAppearance,
    pub loading: ScreenAppearance,
    pub glass: Glass,
}

impl Default for Appearance {
    fn default() -> Self {
        let mut login = ScreenAppearance::default();
        login.logo.size = 0.85;
        let mut loading = ScreenAppearance::default();
        loading.logo.size = 0.18;
        Self {
            login,
            loading,
            glass: Glass::default(),
        }
    }
}

impl Appearance {
    pub fn image(&self, slot: Slot) -> &ImageSettings {
        match slot {
            Slot::LoginBackground => &self.login.background,
            Slot::LoginLogo => &self.login.logo,
            Slot::LoadingBackground => &self.loading.background,
            Slot::LoadingLogo => &self.loading.logo,
        }
    }

    pub fn image_mut(&mut self, slot: Slot) -> &mut ImageSettings {
        match slot {
            Slot::LoginBackground => &mut self.login.background,
            Slot::LoginLogo => &mut self.login.logo,
            Slot::LoadingBackground => &mut self.loading.background,
            Slot::LoadingLogo => &mut self.loading.logo,
        }
    }

    pub fn normalize(&mut self) {
        for slot in [Slot::LoginBackground, Slot::LoginLogo, Slot::LoadingBackground, Slot::LoadingLogo] {
            self.image_mut(slot).normalize();
        }
        self.glass.normalize();
    }
}

/// Editing is isolated from committed preferences. A failed save leaves the
/// previous saved value intact, so Cancel can always restore it.
#[cfg(test)]
#[derive(Debug, Clone)]
struct Draft {
    pub saved: Appearance,
    pub pending: Appearance,
}

#[cfg(test)]
impl Draft {
    pub fn new(saved: Appearance) -> Self {
        Self {
            pending: saved.clone(),
            saved,
        }
    }

    pub fn dirty(&self) -> bool {
        self.pending != self.saved
    }
    pub fn cancel(&mut self) {
        self.pending = self.saved.clone();
    }
    pub fn reset(&mut self) {
        self.pending = Appearance::default();
    }

    fn apply_with(&mut self, publish: impl FnOnce(&Appearance) -> Result<(), Error>) -> Result<(), Error> {
        self.pending.normalize();
        publish(&self.pending)?;
        self.saved = self.pending.clone();
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Unreadable,
    Unsupported,
    LogoFormat,
    TooLarge,
    Dimensions,
    Decode,
    UnsafeName,
    Configuration,
    Write,
}

impl Error {
    /// Callers can translate these keys without interpolating private paths.
    pub fn key(self) -> &'static str {
        match self {
            Self::Unreadable => "branding-error-unreadable",
            Self::Unsupported => "branding-error-unsupported",
            Self::LogoFormat => "branding-error-logo-format",
            Self::TooLarge => "branding-error-too-large",
            Self::Dimensions => "branding-error-dimensions",
            Self::Decode => "branding-error-decode",
            Self::UnsafeName => "branding-error-name",
            Self::Configuration => "branding-error-config",
            Self::Write => "branding-error-write",
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::Unreadable => "The selected image could not be read.",
            Self::Unsupported => "Choose a PNG or JPEG image.",
            Self::LogoFormat => "Choose a PNG image for the logo.",
            Self::TooLarge => "Choose an image file no larger than 16 MiB.",
            Self::Dimensions => "Image dimensions exceed 8192 pixels per side or 16 megapixels.",
            Self::Decode => "The image is corrupt or could not be decoded.",
            Self::UnsafeName => "The saved image selection is invalid.",
            Self::Configuration => "The saved appearance could not be read.",
            Self::Write => "The appearance could not be saved. Check that the profile folder is writable.",
        };
        f.write_str(text)
    }
}
impl std::error::Error for Error {}

fn finite(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

pub fn directory() -> PathBuf {
    crate::settings::config_dir().join("branding")
}

pub fn has_saved() -> bool {
    directory().join(CONFIG).is_file()
}

pub fn load() -> Appearance {
    match load_at(&directory()) {
        Ok(value) => value,
        Err(Error::Unreadable) => Appearance::default(),
        Err(error) => {
            log::warn!("Waters Roux appearance: {error}");
            Appearance::default()
        }
    }
}

pub fn save(value: &Appearance) -> Result<(), Error> {
    save_at(&directory(), value)
}

fn load_at(root: &Path) -> Result<Appearance, Error> {
    let bytes = read_bounded(&root.join(CONFIG), MAX_CONFIG_BYTES)?;
    let mut value: Appearance = serde_json::from_slice(&bytes).map_err(|_| Error::Configuration)?;
    value.normalize();
    Ok(value)
}

fn save_at(root: &Path, value: &Appearance) -> Result<(), Error> {
    let mut value = value.clone();
    value.normalize();
    let bytes = serde_json::to_vec_pretty(&value).map_err(|_| Error::Configuration)?;
    std::fs::create_dir_all(root).map_err(|_| Error::Write)?;
    crate::atomic_file::write(&root.join(CONFIG), &bytes).map_err(|_| Error::Write)
}

fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, Error> {
    let file = std::fs::File::open(path).map_err(|_| Error::Unreadable)?;
    if !file.metadata().map_err(|_| Error::Unreadable)?.is_file() {
        return Err(Error::Unreadable);
    }
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes).map_err(|_| Error::Unreadable)?;
    if bytes.len() as u64 > limit {
        return Err(Error::TooLarge);
    }
    Ok(bytes)
}

fn safe_name(name: &str) -> bool {
    let Some(rest) = name.strip_prefix("art-") else {
        return false;
    };
    let Some((hash, suffix)) = rest.rsplit_once('.') else {
        return false;
    };
    hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()) && matches!(suffix, "png" | "jpg")
}

pub fn custom_path(name: &str) -> Option<PathBuf> {
    let checked = if safe_name(name) {
        Ok(directory().join("images").join(name))
    } else {
        Err(Error::UnsafeName)
    };
    checked.ok()
}

pub fn official_path(slot: Slot) -> Option<PathBuf> {
    let name = slot.official_file();
    let exe = std::env::current_exe().ok().and_then(|exe| exe.parent().map(Path::to_path_buf));
    exe.map(|dir| dir.join("assets/branding").join(name))
        .filter(|path| path.is_file())
        .or_else(|| {
            let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/branding").join(name);
            path.is_file().then_some(path)
        })
}

pub fn resolve(value: &ImageSettings, slot: Slot) -> Option<PathBuf> {
    match &value.choice {
        ImageChoice::Hidden => None,
        ImageChoice::Official => official_path(slot),
        ImageChoice::Custom(name) => custom_path(name).filter(|path| path.is_file()).or_else(|| official_path(slot)),
    }
}

fn validate_bytes(bytes: &[u8], slot: Slot) -> Result<ImageFormat, Error> {
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(Error::TooLarge);
    }
    let format = image::guess_format(bytes).map_err(|_| Error::Unsupported)?;
    if !matches!(format, ImageFormat::Png | ImageFormat::Jpeg) {
        return Err(Error::Unsupported);
    }
    if slot.is_logo() && format != ImageFormat::Png {
        return Err(Error::LogoFormat);
    }
    let (width, height) = ImageReader::with_format(Cursor::new(bytes), format)
        .into_dimensions()
        .map_err(|_| Error::Decode)?;
    if width == 0 || height == 0 || width > MAX_SIDE || height > MAX_SIDE || u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(Error::Dimensions);
    }
    Ok(format)
}

fn decode_bytes(bytes: &[u8], slot: Slot) -> Result<DynamicImage, Error> {
    let format = validate_bytes(bytes, slot)?;
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_SIDE);
    limits.max_image_height = Some(MAX_SIDE);
    limits.max_alloc = Some(96 * 1024 * 1024);
    reader.limits(limits);
    reader.decode().map_err(|_| Error::Decode)
}

pub fn decode(path: &Path, slot: Slot) -> Result<DynamicImage, Error> {
    decode_bytes(&read_bounded(path, MAX_FILE_BYTES)?, slot)
}

pub fn import(path: &Path, slot: Slot) -> Result<String, Error> {
    import_at(&directory(), path, slot)
}

fn import_at(root: &Path, path: &Path, slot: Slot) -> Result<String, Error> {
    let bytes = read_bounded(path, MAX_FILE_BYTES)?;
    let format = validate_bytes(&bytes, slot)?;
    // Full decode before publication also rejects truncated payloads whose
    // headers looked valid. The original encoded bytes are what we store.
    decode_bytes(&bytes, slot)?;
    let digest = Sha256::digest(&bytes);
    let hash: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    let suffix = if format == ImageFormat::Png { "png" } else { "jpg" };
    let name = format!("art-{hash}.{suffix}");
    let images = root.join("images");
    std::fs::create_dir_all(&images).map_err(|_| Error::Write)?;
    crate::atomic_file::write(&images.join(&name), &bytes).map_err(|_| Error::Write)?;
    Ok(name)
}

pub fn load_image(value: &ImageSettings, slot: Slot) -> Option<DynamicImage> {
    let path = resolve(value, slot)?;
    decode(&path, slot).ok().or_else(|| {
        official_path(slot)
            .filter(|official| *official != path)
            .and_then(|official| decode(&official, slot).ok())
    })
}

/// Background placement and texture crop share the exact transform. Enlarged
/// images are clipped safely; Fit leaves letterboxing, Center starts at native
/// pixel size. Alignment picks which portion remains visible when cropped.
pub fn placement(value: &ImageSettings, image: [u32; 2], bounds: Rect) -> (Rect, Rect) {
    let mut value = value.clone();
    value.normalize();
    let size = Vec2::new(image[0].max(1) as f32, image[1].max(1) as f32);
    let ratios = bounds.size() / size;
    let base = match value.fit {
        Fit::Fill => ratios.x.max(ratios.y),
        Fit::Contain => ratios.x.min(ratios.y),
        Fit::Center => 1.0,
    };
    let drawn = size * base * value.scale;
    let origin = bounds.min + (bounds.size() - drawn) * Vec2::new(value.x, value.y);
    let full = Rect::from_min_size(origin, drawn);
    let visible = full.intersect(bounds);
    let uv = Rect::from_min_max(
        Pos2::new((visible.left() - full.left()) / drawn.x, (visible.top() - full.top()) / drawn.y),
        Pos2::new((visible.right() - full.left()) / drawn.x, (visible.bottom() - full.top()) / drawn.y),
    );
    (visible, uv)
}

#[cfg(test)]
mod tests;
