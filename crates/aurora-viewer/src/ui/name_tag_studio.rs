//! Resident-side name-tag designer. Styles never replace server-authoritative
//! avatar names or transmit an image to other residents. Custom backgrounds
//! are local visual preferences, with strict file and frame limits.

use super::widgets::{self, ActionKind, Floater};
use crate::{settings::Settings, theme::Palette};
use egui::{Color32, Context, Pos2, Rect, TextureHandle, TextureOptions, Vec2};
use image::{AnimationDecoder, GenericImageView, ImageDecoder};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, io::Cursor, path::Path};

pub const WINDOW: &str = "waters_roux_nametag_designer";
const MAX_FILE: u64 = 8 * 1024 * 1024;
const MAX_SIDE: u32 = 512;
const MAX_FRAMES: usize = 32;

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
thread_local! {
    static ART: RefCell<Option<Art>> = const { RefCell::new(None) };
}

fn decode_art(ctx: &Context, path: &str) -> Result<Art, String> {
    let file = Path::new(path);
    let meta = std::fs::metadata(file).map_err(|e| format!("Could not read image: {e}"))?;
    if meta.len() > MAX_FILE {
        return Err("Name-tag images must be at most 8 MB".into());
    }
    let bytes = std::fs::read(file).map_err(|e| e.to_string())?;
    let mut frames = Vec::new();
    let mut delays = Vec::new();
    let is_gif = file.extension().is_some_and(|e| e.eq_ignore_ascii_case("gif"));
    if is_gif {
        let decoder = image::codecs::gif::GifDecoder::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
        let (w, h) = decoder.dimensions();
        if w == 0 || h == 0 || w > MAX_SIDE || h > MAX_SIDE {
            return Err("GIF dimensions must not exceed 512×512".into());
        }
        for (i, frame) in decoder.into_frames().take(MAX_FRAMES).enumerate() {
            let frame = frame.map_err(|e| e.to_string())?;
            let (num, den) = frame.delay().numer_denom_ms();
            let seconds = (num as f64 / den.max(1) as f64 / 1000.0).clamp(0.08, 2.0);
            let image = frame.into_buffer();
            let (width, height) = image.dimensions();
            let color = egui::ColorImage::from_rgba_unmultiplied([width as usize, height as usize], image.as_raw());
            frames.push(ctx.load_texture(format!("name-tag-gif-{i}"), color, TextureOptions::LINEAR));
            delays.push(seconds);
        }
    } else {
        let source = image::ImageReader::new(Cursor::new(&bytes))
            .with_guessed_format()
            .map_err(|e| e.to_string())?;
        let (w, h) = source.into_dimensions().map_err(|e| e.to_string())?;
        if w == 0 || h == 0 || w > MAX_SIDE || h > MAX_SIDE {
            return Err("Image dimensions must not exceed 512×512".into());
        }
        let image = image::load_from_memory(&bytes).map_err(|e| e.to_string())?.to_rgba8();
        let color = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], image.as_raw());
        frames.push(ctx.load_texture("name-tag-image", color, TextureOptions::LINEAR));
        delays.push(1.0);
    }
    if frames.is_empty() {
        return Err("The image contains no decodable frames".into());
    }
    let total = delays.iter().sum();
    Ok(Art {
        path: path.into(),
        frames,
        delays,
        total,
    })
}

/// Load once per path and reuse GPU textures; an animated GIF changes its
/// displayed texture over time without repeatedly decoding the file.
pub fn texture(ctx: &Context, style: &Style, now: f64) -> Option<TextureHandle> {
    if !style.enabled || style.image_path.trim().is_empty() {
        return None;
    }
    ART.with_borrow_mut(|cached| {
        if cached.as_ref().is_none_or(|a| a.path != style.image_path) {
            *cached = decode_art(ctx, &style.image_path).ok();
        }
        let art = cached.as_ref()?;
        if art.frames.len() > 1 {
            ctx.request_repaint_after(std::time::Duration::from_millis(80));
        }
        let mut t = now.rem_euclid(art.total.max(0.001));
        let mut index = 0;
        for (i, &duration) in art.delays.iter().enumerate() {
            if t < duration {
                index = i;
                break;
            }
            t -= duration;
        }
        art.frames.get(index).cloned()
    })
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
            let mut color =
                Color32::from_rgba_unmultiplied(style.background[0], style.background[1], style.background[2], style.background[3]);
            if ui.color_edit_button_srgba(&mut color).changed() {
                style.background = [color.r(), color.g(), color.b(), color.a()];
                changed = true;
            }
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
        ui.separator();
        ui.strong("Live preview");
        let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 90.0), egui::Sense::hover());
        ui.painter().rect_filled(r, style.corner_radius, style.color(1.0));
        if let Some(tex) = texture(ctx, style, ui.input(|i| i.time)) {
            ui.painter().image(
                tex.id(),
                r,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE.gamma_multiply(style.image_opacity),
            );
        }
        let font = egui::FontId::proportional(style.name_size);
        ui.painter()
            .text(r.center(), egui::Align2::CENTER_CENTER, "Example Resident", font, Color32::WHITE);
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
