//! Resident-side name-tag designer. Styles never replace server-authoritative
//! avatar names or transmit an image to other residents. Custom backgrounds
//! are local visual preferences, with strict file and frame limits.

use super::widgets::{self, ActionKind, Floater};
use crate::{settings::Settings, theme::Palette};
use egui::{Color32, Context, Pos2, Rect, TextureHandle, TextureOptions, Vec2};
use image::{AnimationDecoder, ImageDecoder};
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::VecDeque,
    io::Cursor,
    path::Path,
    sync::mpsc::{self, Receiver, TryRecvError},
};

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
    /// Ordinary Second Life classic name tags, independent of the custom skin.
    /// Defaults retain the original 13pt/11pt typography and full opacity.
    pub regular_name_size: f32,
    pub regular_detail_size: f32,
    pub regular_opacity: f32,
    pub regular_monospace: bool,
    pub regular_bold: bool,
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
            regular_name_size: 13.0,
            regular_detail_size: 11.0,
            regular_opacity: 1.0,
            regular_monospace: false,
            regular_bold: false,
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
        self.regular_name_size = if self.regular_name_size.is_finite() {
            self.regular_name_size.clamp(10.0, 28.0)
        } else {
            13.0
        };
        self.regular_detail_size = if self.regular_detail_size.is_finite() {
            self.regular_detail_size.clamp(9.0, 24.0)
        } else {
            11.0
        };
        self.regular_opacity = if self.regular_opacity.is_finite() {
            self.regular_opacity.clamp(0.2, 1.0)
        } else {
            1.0
        };
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
        self.border_width = if self.border_width.is_finite() {
            self.border_width.clamp(0.0, 6.0)
        } else {
            1.0
        };
        self.padding = if self.padding.is_finite() {
            self.padding.clamp(2.0, 32.0)
        } else {
            8.0
        };
        self.text_height = if self.text_height.is_finite() {
            self.text_height.clamp(-40.0, 40.0)
        } else {
            0.0
        };
        self.detail_size = if self.detail_size.is_finite() {
            self.detail_size.clamp(9.0, 24.0)
        } else {
            12.0
        };
        if !matches!(self.image_fit.as_str(), "cover" | "contain" | "stretch") {
            self.image_fit = "cover".into();
        }
        if self.image_path.len() > 2048 {
            self.image_path.clear();
        }
    }

    /// Never dim chat bubbles or residents' actual identities; only the
    /// classic name-tag card, glyphs and voice decorations use this factor.
    pub fn regular_fade(&self, distance_fade: f32) -> f32 {
        (distance_fade * self.regular_opacity).clamp(0.0, 1.0)
    }

    pub fn regular_font_size(&self, name: bool) -> f32 {
        if name { self.regular_name_size } else { self.regular_detail_size }
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

/// File I/O and image decoding are done on a worker. GPU uploads stay on the
/// paint thread, but are bounded to two frames per repaint.
struct DecodedArt {
    frames: VecDeque<image::RgbaImage>,
    delays: Vec<f64>,
    total: f64,
}

enum Cache {
    Loading {
        path: String,
        receiver: Receiver<Result<DecodedArt, String>>,
    },
    Uploading {
        path: String,
        frames: VecDeque<image::RgbaImage>,
        uploaded: Vec<TextureHandle>,
        delays: Vec<f64>,
        total: f64,
    },
    Ready(Art),
    Failed {
        path: String,
        reason: String,
    },
}

impl Cache {
    fn path(&self) -> &str {
        match self {
            Self::Loading { path, .. } | Self::Uploading { path, .. } | Self::Failed { path, .. } => path,
            Self::Ready(art) => &art.path,
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

// Windows Explorer's Copy as path includes surrounding quotes. Accept that
// form alongside paths selected from the picker and drag-and-drop.
fn local_image_path(path: &str) -> &str {
    let trimmed = path.trim();
    trimmed.strip_prefix('"').and_then(|s| s.strip_suffix('"')).unwrap_or(trimmed)
}

fn decode_art(path: &str) -> Result<DecodedArt, String> {
    let file = Path::new(local_image_path(path));
    let meta = std::fs::metadata(file).map_err(|e| format!("Cannot open file: {e}"))?;
    if meta.len() > MAX_FILE {
        return Err("The image exceeds 32 MB. Compress it before importing.".into());
    }
    let ext = file.extension().and_then(|v| v.to_str()).unwrap_or_default().to_ascii_lowercase();
    if !matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif") {
        return Err("Choose a PNG, JPG, JPEG, or GIF file.".into());
    }
    let bytes = std::fs::read(file).map_err(|e| format!("Cannot read image: {e}"))?;
    let mut frames = VecDeque::new();
    let mut delays = Vec::new();
    if ext == "gif" {
        let decoder = image::codecs::gif::GifDecoder::new(Cursor::new(bytes)).map_err(|e| format!("Invalid GIF: {e}"))?;
        let (w, h) = decoder.dimensions();
        if !fits_size(w, h, MAX_GIF_SIDE) {
            return Err("GIF dimensions must be at most 2048 × 2048 pixels.".into());
        }
        for frame in decoder.into_frames().take(MAX_FRAMES) {
            let frame = frame.map_err(|e| format!("GIF frame could not be decoded: {e}"))?;
            let (num, den) = frame.delay().numer_denom_ms();
            let seconds = (num as f64 / den.max(1) as f64 / 1000.0).clamp(0.08, 2.0);
            let rgba = shrink(frame.into_buffer());
            frames.push_back(rgba);
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
        let rgba = image::load_from_memory(&bytes)
            .map_err(|e| format!("Cannot decode image: {e}"))?
            .to_rgba8();
        frames.push_back(shrink(rgba));
        delays.push(1.0);
    }
    if frames.is_empty() {
        return Err("The file has no readable image frames.".into());
    }
    let total = delays.iter().sum();
    Ok(DecodedArt { frames, delays, total })
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
            // Never decode large image assets on the render thread. When a
            // resident switches paths, the previous receiver is dropped; a
            // stale worker can no longer overwrite the new selection.
            let path = style.image_path.clone();
            let worker_path = path.clone();
            let worker_ctx = ctx.clone();
            let (sender, receiver) = mpsc::sync_channel(1);
            let started = std::thread::Builder::new().name("waters-roux-nametag-art".into()).spawn(move || {
                let result = decode_art(&worker_path);
                let _ = sender.send(result);
                worker_ctx.request_repaint();
            });
            *cache = Some(match started {
                Ok(_) => Cache::Loading { path, receiver },
                Err(e) => Cache::Failed {
                    path,
                    reason: format!("Cannot load image in background: {e}"),
                },
            });
        }

        let completed = match cache.as_ref() {
            Some(Cache::Loading { receiver, .. }) => match receiver.try_recv() {
                Ok(result) => Some(result),
                Err(TryRecvError::Disconnected) => Some(Err("Background image loader stopped unexpectedly.".into())),
                Err(TryRecvError::Empty) => None,
            },
            _ => None,
        };
        if let Some(result) = completed {
            *cache = Some(match result {
                Ok(decoded) => Cache::Uploading {
                    path: style.image_path.clone(),
                    frames: decoded.frames,
                    uploaded: Vec::with_capacity(decoded.delays.len()),
                    delays: decoded.delays,
                    total: decoded.total,
                },
                Err(reason) => Cache::Failed {
                    path: style.image_path.clone(),
                    reason,
                },
            });
        }

        // Stagger texture creation so an animated GIF never uploads all of
        // its frames in a single paint. The preview appears once all frames
        // are ready, then follows the original animation timestamps.
        let mut upload_finished = false;
        if let Some(Cache::Uploading { frames, uploaded, .. }) = cache.as_mut() {
            for _ in 0..2 {
                let Some(frame) = frames.pop_front() else { break };
                uploaded.push(to_texture(ctx, format!("waters-roux-name-tag-frame-{}", uploaded.len()), &frame));
            }
            upload_finished = frames.is_empty();
            if !upload_finished {
                ctx.request_repaint();
            }
        }
        if upload_finished
            && let Some(Cache::Uploading {
                path,
                uploaded,
                delays,
                total,
                ..
            }) = cache.take()
        {
            *cache = Some(Cache::Ready(Art {
                path,
                frames: uploaded,
                delays,
                total,
            }));
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
            return (
                destination,
                Rect::from_min_max(Pos2::new(0.5 - half, 0.0), Pos2::new(0.5 + half, 1.0)),
            );
        }
        let half = image_aspect / frame_aspect * 0.5;
        return (
            destination,
            Rect::from_min_max(Pos2::new(0.0, 0.5 - half), Pos2::new(1.0, 0.5 + half)),
        );
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
    let mut clear_image = false;
    let tab_id = egui::Id::unique("waters-roux-nametag-studio-tab");
    let mut tab = ctx.data(|d| d.get_temp::<usize>(tab_id)).unwrap_or(0);
    let mut floater = Floater::new(
        WINDOW,
        "Waters Roux • Name Tag Studio",
        egui::pos2(520.0, 110.0),
        Vec2::new(520.0, 565.0),
    );
    floater.min_size = Vec2::new(425.0, 420.0);
    floater.show(ctx, p, open, |ui| {
        ui.label("Design every visible part of avatar name tags on your viewer.");
        changed |= widgets::switch_labeled(ui, p, &mut style.enabled, "Enable my custom name tags").changed();
        widgets::tabs(ui, p, &mut tab, &[("Typography", true), ("Background", true), ("Layout", true)]);
        widgets::divider(ui, p);
        egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| match tab {
            0 => {
                ui.collapsing("Regular Second Life name tags (without custom skin)", |ui| {
                    ui.label("These settings apply to classic name tags even when custom name tags are disabled.");
                    changed |= ui
                        .add(egui::Slider::new(&mut style.regular_name_size, 10.0..=28.0).text("Display-name font size"))
                        .changed();
                    changed |= ui
                        .add(egui::Slider::new(&mut style.regular_detail_size, 9.0..=24.0).text("Group titles, usernames and details"))
                        .changed();
                    changed |= ui
                        .add(egui::Slider::new(&mut style.regular_opacity, 0.2..=1.0).text("Name tag opacity"))
                        .changed();
                    changed |= widgets::switch_labeled(ui, p, &mut style.regular_monospace, "Monospace classic font").changed();
                    changed |= widgets::switch_labeled(ui, p, &mut style.regular_bold, "Bold classic name").changed();
                    ui.small(
                        "Default: original Second Life font size and full opacity. Text stays local and server names are never changed.",
                    );
                });
                widgets::divider(ui, p);
                ui.strong("Custom Name Tag Studio typography");
                changed |= ui
                    .add(egui::Slider::new(&mut style.name_size, 10.0..=28.0).text("Display-name size"))
                    .changed();
                changed |= ui
                    .add(egui::Slider::new(&mut style.detail_size, 9.0..=24.0).text("Titles and detail size"))
                    .changed();
                changed |= widgets::switch_labeled(ui, p, &mut style.monospace, "Monospace lettering").changed();
                changed |= widgets::switch_labeled(ui, p, &mut style.bold, "Bold name").changed();
                changed |= widgets::switch_labeled(ui, p, &mut style.outline, "Text outline / readability").changed();
                changed |= widgets::switch_labeled(ui, p, &mut style.custom_text_colors, "Custom text colors").changed();
                ui.horizontal(|ui| {
                    ui.label("Display name");
                    changed |= widgets::color_button(ui, p, "tag-name-color", &mut style.name_color, true).changed();
                    ui.label("Other text");
                    changed |= widgets::color_button(ui, p, "tag-detail-color", &mut style.detail_color, true).changed();
                });
                ui.horizontal(|ui| {
                    ui.label("Text outline color");
                    changed |= widgets::color_button(ui, p, "tag-outline-color", &mut style.outline_color, true).changed();
                });
                widgets::divider(ui, p);
                ui.strong("Which information appears");
                changed |= widgets::switch_labeled(ui, p, &mut style.show_group_title, "Group titles").changed();
                changed |= widgets::switch_labeled(ui, p, &mut style.show_username, "Usernames under display names").changed();
                changed |= widgets::switch_labeled(ui, p, &mut style.show_status, "Away / busy / typing indicators").changed();
            }
            1 => {
                ui.strong("Background color and image");
                ui.horizontal(|ui| {
                    ui.label("Background color and transparency");
                    changed |= widgets::color_button(ui, p, "nametag-background-color", &mut style.background, true).changed();
                });
                ui.label("PNG, JPG, JPEG or animated GIF. Up to 32 MB. Large still images are safely resized.");
                ui.horizontal_wrapped(|ui| {
                    if widgets::action_button(ui, p, "folder", "Import image", ActionKind::Primary, Vec2::new(0.0, 30.0)).clicked() {
                        browse = true;
                    }
                    if widgets::action_button(ui, p, "trash", "Remove image", ActionKind::Flat, Vec2::new(0.0, 30.0)).clicked() {
                        clear_image = true;
                    }
                });
                changed |= ui.text_edit_singleline(&mut style.image_path).changed();
                ui.horizontal(|ui| {
                    ui.label("Image fit");
                    changed |= ui
                        .selectable_value(&mut style.image_fit, "cover".to_owned(), "Fill / crop")
                        .changed();
                    changed |= ui.selectable_value(&mut style.image_fit, "contain".to_owned(), "Fit").changed();
                    changed |= ui.selectable_value(&mut style.image_fit, "stretch".to_owned(), "Stretch").changed();
                });
                changed |= ui
                    .add(egui::Slider::new(&mut style.image_opacity, 0.0..=1.0).text("Image opacity"))
                    .changed();
                if !style.image_path.is_empty() {
                    if let Some(error) = image_error() {
                        ui.label(egui::RichText::new(format!("Image import failed: {error}")).color(p.danger));
                    } else {
                        ui.label(egui::RichText::new("Image loaded or awaiting preview").color(p.success));
                    }
                }
                ui.label("Tip: you can also drag a local PNG, JPG or GIF onto this window.");
            }
            _ => {
                ui.strong("Tag shape and position");
                changed |= ui
                    .add(egui::Slider::new(&mut style.padding, 2.0..=32.0).text("Space around name"))
                    .changed();
                changed |= ui
                    .add(egui::Slider::new(&mut style.corner_radius, 0.0..=20.0).text("Rounded corners"))
                    .changed();
                changed |= ui
                    .add(egui::Slider::new(&mut style.border_width, 0.0..=6.0).text("Border thickness"))
                    .changed();
                changed |= ui
                    .add(egui::Slider::new(&mut style.text_height, -40.0..=40.0).text("Height above avatar"))
                    .changed();
                ui.horizontal(|ui| {
                    ui.label("Border color");
                    changed |= widgets::color_button(ui, p, "tag-border-color", &mut style.border_color, true).changed();
                });
                widgets::divider(ui, p);
                ui.strong("Viewer-wide visibility");
                changed |= widgets::switch_labeled(ui, p, &mut settings.show_own_name_tag, "My own name tag").changed();
                changed |= widgets::switch_labeled(ui, p, &mut settings.show_friend_name_tags, "Friends' name tags").changed();
                changed |= widgets::switch_labeled(ui, p, &mut settings.show_other_name_tags, "Other residents' name tags").changed();
            }
        });
        widgets::divider(ui, p);
        ui.strong("Live preview");
        let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 92.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, style.corner_radius, style.color(1.0));
        if let Some(texture) = texture(ctx, style, ui.input(|i| i.time)) {
            let (dest, uv) = image_rect(&texture, rect, &style.image_fit);
            ui.painter()
                .image(texture.id(), dest, uv, widgets::PICTURE_TINT.gamma_multiply(style.image_opacity));
        }
        if style.border_width > 0.0 {
            let [r, g, b, a] = style.border_color;
            ui.painter().rect_stroke(
                rect,
                style.corner_radius,
                egui::Stroke::new(style.border_width, Color32::from_rgba_unmultiplied(r, g, b, a)),
                egui::StrokeKind::Inside,
            );
        }
        let font = egui::FontId::new(
            style.name_size,
            if style.monospace {
                egui::FontFamily::Monospace
            } else {
                egui::FontFamily::Proportional
            },
        );
        let [r, g, b, a] = style.name_color;
        let text = if style.custom_text_colors {
            Color32::from_rgba_unmultiplied(r, g, b, a)
        } else {
            p.ink
        };
        ui.painter()
            .text(rect.center(), egui::Align2::CENTER_CENTER, "Example Resident", font, text);
        if widgets::action_button(ui, p, "refresh", "Restore defaults", ActionKind::Flat, Vec2::new(0.0, 29.0)).clicked() {
            reset = true;
        }
        ui.label("These settings only affect your local display; they never change actual Second Life identities.");
    });
    ctx.data_mut(|d| d.insert_temp(tab_id, tab));
    if browse
        && let Some(path) = rfd::FileDialog::new()
            .add_filter("Name tag images", &["png", "jpg", "jpeg", "gif"])
            .pick_file()
    {
        style.image_path = path.to_string_lossy().into_owned();
        style.enabled = true;
        changed = true;
    }
    if let Some(path) = ctx.input(|i| {
        i.raw.dropped_files.iter().find_map(|f| {
            let path = f.path();
            (!path.as_os_str().is_empty()
                && path
                    .extension()
                    .is_some_and(|e| matches!(e.to_string_lossy().to_ascii_lowercase().as_str(), "png" | "jpg" | "jpeg" | "gif")))
            .then(|| path.to_path_buf())
        })
    }) {
        style.image_path = path.to_string_lossy().into_owned();
        style.enabled = true;
        changed = true;
    }
    if clear_image {
        style.image_path.clear();
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
    fn accepts_windows_explorer_quoted_image_paths_without_replacing_filenames() {
        assert_eq!(
            local_image_path(r#"  "C:\\Users\\Creator\\Pictures\\my nametag.PNG"  "#),
            r"C:\\Users\\Creator\\Pictures\\my nametag.PNG"
        );
        assert_eq!(local_image_path("C:/Pictures/tag.jpg"), "C:/Pictures/tag.jpg");
        assert_eq!(local_image_path(""), "");
    }

    #[test]
    fn png_jpg_and_gif_actually_decode_as_name_tag_backgrounds() {
        use std::{io::Cursor, path::PathBuf};
        let root: PathBuf = std::env::temp_dir().join(format!("waters-roux-nametag-import-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("create isolated import fixture");
        for (ext, format) in [
            ("png", image::ImageFormat::Png),
            ("jpg", image::ImageFormat::Jpeg),
            ("gif", image::ImageFormat::Gif),
        ] {
            let still = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(3, 2, image::Rgb([210, 120, 140])));
            let mut encoded = Cursor::new(Vec::new());
            still.write_to(&mut encoded, format).expect("encode tiny valid image");
            let file = root.join(format!("creator-tag.{ext}"));
            std::fs::write(&file, encoded.into_inner()).expect("write import fixture");
            let path = format!(r#""{}""#, file.display());
            let art = decode_art(&path).expect("decode PNG, JPG or GIF image");
            assert!(!art.frames.is_empty(), "{ext} must load at least one image frame");
            assert!(art.frames[0].width() > 0 && art.frames[0].height() > 0);
        }
        std::fs::remove_dir_all(root).expect("remove isolated import fixture");
    }

    #[test]
    fn classic_name_tag_style_keeps_legacy_defaults_and_roundtrips() {
        let from_old: Style = serde_json::from_str("{}").expect("older name-tag settings");
        assert!(!from_old.enabled);
        assert_eq!(from_old.regular_name_size, 13.0);
        assert_eq!(from_old.regular_detail_size, 11.0);
        assert_eq!(from_old.regular_opacity, 1.0);
        assert_eq!(from_old.regular_font_size(true), 13.0);
        assert_eq!(from_old.regular_font_size(false), 11.0);

        let edited = Style {
            regular_name_size: 23.0,
            regular_detail_size: 14.0,
            regular_opacity: 0.65,
            regular_monospace: true,
            regular_bold: true,
            ..from_old
        };
        let restored: Style = serde_json::from_str(&serde_json::to_string(&edited).unwrap()).unwrap();
        assert_eq!(restored, edited);
        assert_eq!(restored.regular_fade(0.5), 0.325);
    }

    #[test]
    fn classic_tags_clamp_invalid_values_without_changing_the_custom_skin() {
        let mut style = Style {
            regular_name_size: f32::NAN,
            regular_detail_size: 400.0,
            regular_opacity: -3.0,
            name_size: 19.0,
            ..Style::default()
        };
        style.sanitize();
        assert_eq!(style.regular_name_size, 13.0);
        assert_eq!(style.regular_detail_size, 24.0);
        assert_eq!(style.regular_opacity, 0.2);
        assert_eq!(style.name_size, 19.0);
    }

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
