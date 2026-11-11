//! Resident-side name-tag designer. Styles never replace server-authoritative
//! avatar names or transmit an image to other residents. Custom backgrounds
//! are local visual preferences, with strict file and frame limits.

use super::widgets::{self, ActionKind, Floater};
use crate::{settings::Settings, theme::Palette};
use egui::{Color32, Context, Pos2, Rect, TextureHandle, TextureOptions, Vec2};
use image::{AnimationDecoder, ImageDecoder};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, io::Cursor, path::Path};

pub const WINDOW: &str = "waters_roux_nametag_designer";
// Original assets may be high-resolution; scale to a bounded GPU texture
// instead of rejecting every ordinary creator-sized PNG/JPG/GIF.
const MAX_FILE: u64 = 32 * 1024 * 1024;
const MAX_IMAGE_SIDE: u32 = 8192;
const MAX_GIF_SIDE: u32 = 2048;
const MAX_TEXTURE_SIDE: u32 = 1024;
const MAX_FRAMES: usize = 48;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Style {
    pub enabled: bool,
    pub name_size: f32,
    pub bold: bool,
    pub outline: bool,
    pub background: [u8; 4],
    pub image_path: String,
    pub image_opacity: f32,
    pub corner_radius: f32,
    pub name_color: [u8; 4],
    pub detail_color: [u8; 4],
    pub border_color: [u8; 4],
    pub outline_color: [u8; 4],
    pub border_width: f32,
    pub padding: f32,
    pub text_height: f32,
    pub detail_size: f32,
    pub monospace: bool,
    pub custom_text_colors: bool,
    pub show_group_title: bool,
    pub show_username: bool,
    pub show_status: bool,
    /// "cover" crops to fill, "contain" preserves the entire image,
    /// "stretch" fills the name tag without preserving aspect ratio.
    pub image_fit: String,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            enabled: false,
            name_size: 16.0,
            bold: true,
            outline: true,
            background: [24, 12, 24, 225],
            image_path: String::new(),
            image_opacity: 0.95,
            corner_radius: 8.0,
            name_color: [255, 255, 255, 255],
            detail_color: [224, 219, 231, 255],
            border_color: [209, 160, 237, 255],
            outline_color: [20, 13, 30, 255],
            border_width: 1.0,
            padding: 8.0,
            text_height: 0.0,
            detail_size: 12.0,
            monospace: false,
            custom_text_colors: false,
            show_group_title: true,
            show_username: true,
            show_status: true,
            image_fit: "cover".into(),
        }
    }
}

impl Style {
    pub fn sanitize(&mut self) {
        self.name_size = if self.name_size.is_finite() {
            self.name_size.clamp(10.0, 28.0)
        } else {
            16.0
        };
        self.corner_radius = if self.corner_radius.is_finite() {
            self.corner_radius.clamp(0.0, 20.0)
        } else {
            8.0
        };
        self.image_opacity = if self.image_opacity.is_finite() {
            self.image_opacity.clamp(0.0, 1.0)
        } else {
            1.0
        };
        self.border_width = if self.border_width.is_finite() { self.border_width.clamp(0.0, 6.0) } else { 1.0 };
        self.padding = if self.padding.is_finite() { self.padding.clamp(2.0, 32.0) } else { 8.0 };
        self.text_height = if self.text_height.is_finite() { self.text_height.clamp(-40.0, 40.0) } else { 0.0 };
        self.detail_size = if self.detail_size.is_finite() { self.detail_size.clamp(9.0, 24.0) } else { 12.0 };
        if !matches!(self.image_fit.as_str(), "cover" | "contain" | "stretch") {
            self.image_fit = "cover".into();
        }
        if self.image_path.len() > 2048 {
            self.image_path.clear();
        }
    }

    pub fn color(&self, fade: f32) -> Color32 {
        let [r, g, b, a] = self.background;
        Color32::from_rgba_unmultiplied(r, g, b, (a as f32 * fade.clamp(0.0, 1.0)) as u8)
    }
}

struct Art {
    path: String,
    frames: Vec<TextureHandle>,
    delays: Vec<f64>,
    total: f64,
}

enum Cache {
    Ready(Art),
    Failed { path: String, reason: String },
}

impl Cache {
    fn path(&self) -> &str {
        match self {
            Self::Ready(art) => &art.path,
            Self::Failed { path, .. } => path,
        }
    }
}

thread_local! {
    static ART: RefCell<Option<Cache>> = const { RefCell::new(None) };
}

fn fits_size(width: u32, height: u32, max: u32) -> bool {
    width > 0 && height > 0 && width <= max && height <= max
}

fn to_texture(ctx: &Context, key: String, rgba: &image::RgbaImage) -> TextureHandle {
    let (w, h) = rgba.dimensions();
    let size = [w as usize, h as usize];
    let pixels = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
    ctx.load_texture(key, pixels, TextureOptions::LINEAR)
}

fn shrink(image: image::RgbaImage) -> image::RgbaImage {
    let (w, h) = image.dimensions();
    if w <= MAX_TEXTURE_SIDE && h <= MAX_TEXTURE_SIDE {
        return image;
    }
    let scale = (MAX_TEXTURE_SIDE as f32 / w as f32).min(MAX_TEXTURE_SIDE as f32 / h as f32);
    let width = (w as f32 * scale).round().max(1.0) as u32;
    let height = (h as f32 * scale).round().max(1.0) as u32;
    image::imageops::resize(&image, width, height, image::imageops::FilterType::Lanczos3)
}

fn decode_art(ctx: &Context, path: &str) -> Result<Art, String> {
    let file = Path::new(path);
    let meta = std::fs::metadata(file).map_err(|e| format!("Cannot open file: {e}"))?;
    if meta.len() > MAX_FILE {
        return Err("The image exceeds 32 MB. Compress it before importing.".into());
    }
    let ext = file.extension().and_then(|v| v.to_str()).unwrap_or_default().to_ascii_lowercase();
    if !matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif") {
        return Err("Choose a PNG, JPG, JPEG, or GIF file.".into());
    }
    let bytes = std::fs::read(file).map_err(|e| format!("Cannot read image: {e}"))?;
    let mut frames = Vec::new();
    let mut delays = Vec::new();
    if ext == "gif" {
        let decoder = image::codecs::gif::GifDecoder::new(Cursor::new(bytes)).map_err(|e| format!("Invalid GIF: {e}"))?;
        let (w, h) = decoder.dimensions();
        if !fits_size(w, h, MAX_GIF_SIDE) {
            return Err("GIF dimensions must be at most 2048 × 2048 pixels.".into());
        }
        for (i, frame) in decoder.into_frames().take(MAX_FRAMES).enumerate() {
            let frame = frame.map_err(|e| format!("GIF frame could not be decoded: {e}"))?;
            let (num, den) = frame.delay().numer_denom_ms();
            let seconds = (num as f64 / den.max(1) as f64 / 1000.0).clamp(0.08, 2.0);
            let rgba = shrink(frame.into_buffer());
            frames.push(to_texture(ctx, format!("waters-roux-name-tag-gif-{i}"), &rgba));
            delays.push(seconds);
        }
    } else {
        let source = image::ImageReader::new(Cursor::new(&bytes))
            .with_guessed_format()
            .map_err(|e| e.to_string())?;
        let (w, h) = source.into_dimensions().map_err(|e| format!("Cannot read image dimensions: {e}"))?;
        if !fits_size(w, h, MAX_IMAGE_SIDE) {
            return Err("Image dimensions must be at most 8192 × 8192 pixels.".into());
        }
        let rgba = image::load_from_memory(&bytes).map_err(|e| format!("Cannot decode image: {e}"))?.to_rgba8();
        frames.push(to_texture(ctx, "waters-roux-name-tag-background".into(), &shrink(rgba)));
        delays.push(1.0);
    }
    if frames.is_empty() {
        return Err("The file has no readable image frames.".into());
    }
    let total = delays.iter().sum();
    Ok(Art { path: path.into(), frames, delays, total })
}

/// A visible import error instead of silently discarding ordinary PNG/JPG/GIFs.
/// Invalid paths are cached so the painter never repeatedly opens files.
pub fn image_error() -> Option<String> {
    ART.with_borrow(|cached| match cached.as_ref() {
        Some(Cache::Failed { reason, .. }) => Some(reason.clone()),
        _ => None,
    })
}

/// Return the GPU texture for the requested image or current GIF frame.
pub fn texture(ctx: &Context, style: &Style, now: f64) -> Option<TextureHandle> {
    if !style.enabled || style.image_path.trim().is_empty() {
        return None;
    }
    ART.with_borrow_mut(|cache| {
        if cache.as_ref().is_none_or(|asset| asset.path() != style.image_path) {
            *cache = Some(match decode_art(ctx, &style.image_path) {
                Ok(art) => Cache::Ready(art),
                Err(reason) => Cache::Failed { path: style.image_path.clone(), reason },
            });
        }
        let Some(Cache::Ready(art)) = cache.as_ref() else {
            return None;
        };
        if art.frames.len() > 1 {
            ctx.request_repaint_after(std::time::Duration::from_millis(80));
        }
        let mut time = now.rem_euclid(art.total.max(0.001));
        let mut index = art.frames.len().saturating_sub(1);
        for (i, &delay) in art.delays.iter().enumerate() {
            if time < delay {
                index = i;
                break;
            }
            time -= delay;
        }
        art.frames.get(index).cloned()
    })
}

/// Returns destination and UV rectangles for crop/contain/stretch, so imported
/// backgrounds do not distort unless the resident explicitly chooses Stretch.
pub fn image_rect(texture: &TextureHandle, destination: Rect, fit: &str) -> (Rect, Rect) {
    let [w, h] = texture.size();
    let image_aspect = w as f32 / h.max(1) as f32;
    let frame_aspect = destination.width() / destination.height().max(1.0);
    let full = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
    if fit == "contain" {
        let size = if image_aspect > frame_aspect {
            Vec2::new(destination.width(), destination.width() / image_aspect)
        } else {
            Vec2::new(destination.height() * image_aspect, destination.height())
        };
        return (Rect::from_center_size(destination.center(), size), full);
    }
    if fit == "cover" {
        if image_aspect > frame_aspect {
            let half = frame_aspect / image_aspect * 0.5;
            return (destination, Rect::from_min_max(Pos2::new(0.5 - half, 0.0), Pos2::new(0.5 + half, 1.0)));
        }
        let half = image_aspect / frame_aspect * 0.5;
        return (destination, Rect::from_min_max(Pos2::new(0.0, 0.5 - half), Pos2::new(1.0, 0.5 + half)));
    }
    (destination, full)
}

pub fn show(ctx: &Context, p: &Palette, open: &mut bool, settings: &mut Settings) -> bool {
    if !*open {
        return false;
    }
    let mut changed = false;
    let style = &mut settings.name_tag_style;
    let mut reset = false;
    let mut browse = false;
    let mut floater = Floater::new(
        WINDOW,
        "Waters Roux Name Tag Studio",
        egui::pos2(550.0, 120.0),
        Vec2::new(445.0, 515.0),
    );
    floater.min_size = Vec2::new(360.0, 380.0);
    floater.show(ctx, p, open, |ui| {
        ui.label("Customize how nearby avatar name tags appear on YOUR viewer.");
        changed |= widgets::switch_labeled(ui, p, &mut style.enabled, "Use my custom name-tag design").changed();
        ui.add_enabled_ui(style.enabled, |ui| {
            changed |= ui
                .add(egui::Slider::new(&mut style.name_size, 10.0..=28.0).text("Name font size"))
                .changed();
            changed |= widgets::switch_labeled(ui, p, &mut style.bold, "Bold names").changed();
            changed |= widgets::switch_labeled(ui, p, &mut style.outline, "Text readability outline").changed();
            changed |= ui
                .add(egui::Slider::new(&mut style.corner_radius, 0.0..=20.0).text("Corner radius"))
                .changed();
            changed |= widgets::color_button(ui, p, "nametag-background-color", &mut style.background, true).changed();
            ui.label("Background image: local PNG or animated GIF (up to 8 MB, 512×512)");
            ui.horizontal(|ui| {
                changed |= ui.text_edit_singleline(&mut style.image_path).changed();
                if widgets::action_button(ui, p, "folder", "Browse", ActionKind::Flat, Vec2::new(0.0, 28.0)).clicked() {
                    browse = true;
                }
            });
            changed |= ui
                .add(egui::Slider::new(&mut style.image_opacity, 0.0..=1.0).text("Image opacity"))
                .changed();
        });
        widgets::divider(ui, p);
        ui.strong("Live preview");
        let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 90.0), egui::Sense::hover());
        ui.painter().rect_filled(r, style.corner_radius, style.color(1.0));
        if let Some(tex) = texture(ctx, style, ui.input(|i| i.time)) {
            ui.painter().image(
                tex.id(),
                r,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                widgets::PICTURE_TINT.gamma_multiply(style.image_opacity),
            );
        }
        let font = egui::FontId::proportional(style.name_size);
        ui.painter()
            .text(r.center(), egui::Align2::CENTER_CENTER, "Example Resident", font, p.ink);
        if widgets::action_button(ui, p, "refresh", "Restore defaults", ActionKind::Flat, Vec2::new(0.0, 29.0)).clicked() {
            reset = true;
        }
        ui.label("Names and usernames remain authoritative Second Life identities. Image designs only affect your local display.");
    });
    if browse
        && let Some(path) = rfd::FileDialog::new()
            .add_filter("Name tag background", &["png", "gif"])
            .pick_file()
    {
        style.image_path = path.to_string_lossy().into_owned();
        changed = true;
    }
    if reset {
        *style = Style::default();
        changed = true;
    }
    if changed {
        style.sanitize();
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn styles_sanitize_unsafe_values() {
        let mut style = Style {
            name_size: f32::NAN,
            image_opacity: f32::INFINITY,
            corner_radius: -12.0,
            ..Style::default()
        };
        style.sanitize();
        assert_eq!(style.name_size, 16.0);
        assert_eq!(style.image_opacity, 1.0);
        assert_eq!(style.corner_radius, 0.0);
    }
}
