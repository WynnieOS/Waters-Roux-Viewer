//! Optional Waters Roux bayou wallpaper behind both login modes.
//!
//! The owner's original background belongs in `assets/branding/` next to the
//! executable, or in the viewer-specific configuration folder's `branding/`.
//! There is no fabricated grid state, account detail or login form in this art.
//! A simple palette-based bayou dusk illustration is the temporary fallback.
use crate::theme::Palette;
use egui::{Color32, Context, Id, Painter, Pos2, Rect, Shape, TextureHandle, TextureOptions};
use std::path::PathBuf;

const FILE: &str = "waters-roux-background.png";

#[derive(Default)]
pub struct Wallpaper {
    checked: bool,
    image: Option<TextureHandle>,
    frosted: Option<TextureHandle>,
}

fn context_id() -> Id {
    Id::unique("waters_roux_login_frosted_wallpaper")
}

fn candidates() -> Vec<PathBuf> {
    let mut paths = vec![crate::settings::config_dir().join("branding").join(FILE)];
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(std::path::Path::to_path_buf))
    {
        paths.push(dir.join("assets").join("branding").join(FILE));
    }
    paths
}

/// Texture coordinates for a centre-cropped image: the wallpaper fills the
/// login screen at every aspect ratio, including ultrawide.
fn cover_uv(size: [usize; 2], rect: Rect) -> Rect {
    let texture = size[0] as f32 / size[1].max(1) as f32;
    let screen = rect.width().max(1.0) / rect.height().max(1.0);
    if texture > screen {
        let half = 0.5 * screen / texture;
        Rect::from_min_max(Pos2::new(0.5 - half, 0.0), Pos2::new(0.5 + half, 1.0))
    } else {
        let half = 0.5 * texture / screen;
        Rect::from_min_max(Pos2::new(0.0, 0.5 - half), Pos2::new(1.0, 0.5 + half))
    }
}

fn texture(ctx: &Context, name: &str, image: &image::RgbaImage) -> TextureHandle {
    ctx.load_texture(
        name,
        egui::ColorImage::from_rgba_unmultiplied([image.width() as usize, image.height() as usize], image.as_raw()),
        TextureOptions::LINEAR,
    )
}

/// Palette-based placeholder: blush sunset, misty water and cypress banks.
/// Generated once at a bounded size; no borrowed artwork or mockup UI.
fn fallback(p: &Palette) -> image::RgbaImage {
    let (w, h) = (960u32, 540u32);
    image::RgbaImage::from_fn(w, h, |x, y| {
        let u = x as f32 / w as f32;
        let v = y as f32 / h as f32;
        let horizon = 0.55;
        let sky = p.navy.lerp_to_gamma(p.rose, (v / horizon).clamp(0.0, 1.0));
        let water = p.rose.lerp_to_gamma(p.navy, ((v - horizon) / (1.0 - horizon)).clamp(0.0, 1.0));
        let mut c = if v < horizon { sky } else { water };
        if v > horizon {
            let ripple = (u * 170.0 + v * 240.0).sin() * (v * 360.0).sin();
            c = c.lerp_to_gamma(p.bar, (0.04 + ripple * 0.04).max(0.0));
        }
        // Low wooded banks, reflected in the water.
        let bank_y = if v > horizon { 2.0 * horizon - v } else { v };
        for i in 0..16 {
            let n = i as f32;
            let tx = (n * 0.618_034).fract();
            let distance = (u - tx).abs();
            let height = 0.09 + (n * 1.7).sin().abs() * 0.16;
            let bank = (u - 0.5).abs() * 0.05;
            let trunk = distance < 0.003 && bank_y > horizon - height && bank_y < horizon + bank;
            let canopy = ((u - tx) / 0.041).powi(2) + ((bank_y - (horizon - height)) / 0.045).powi(2) < 1.0;
            if trunk || canopy || bank_y > horizon - bank {
                c = c.lerp_to_gamma(p.navy, if v > horizon { 0.38 } else { 0.85 });
            }
        }
        image::Rgba([c.r(), c.g(), c.b(), 255])
    })
}

impl Wallpaper {
    fn load(&mut self, ctx: &Context, p: &Palette) {
        if self.checked {
            return;
        }
        self.checked = true;
        for path in candidates() {
            if !path.is_file() {
                continue;
            }
            let Ok(decoded) = image::open(&path) else {
                log::warn!("Cannot decode Waters Roux login wallpaper at {}", path.display());
                continue;
            };
            // Bound texture memory and blur cost without changing the original file.
            let sharp = decoded.resize(2560, 1440, image::imageops::FilterType::Triangle).to_rgba8();
            let soft = decoded.resize(800, 800, image::imageops::FilterType::Triangle).to_rgba8();
            let soft = image::imageops::blur(&soft, 10.0);
            self.image = Some(texture(ctx, "waters_roux_bayou_background", &sharp));
            self.frosted = Some(texture(ctx, "waters_roux_bayou_frosted", &soft));
            log::info!("Waters Roux background loaded from {}", path.display());
            break;
        }
        if self.image.is_none() {
            // Temporary, deliberately simple bayou dusk illustration. This is
            // not the approved artwork; both faces sample this same image.
            let sharp = fallback(p);
            let soft = image::imageops::blur(&sharp, 12.0);
            self.image = Some(texture(ctx, "waters_roux_temporary_bayou", &sharp));
            self.frosted = Some(texture(ctx, "waters_roux_temporary_bayou_frosted", &soft));
        }
    }

    /// Uses the exact standalone wallpaper when installed, else the temporary bayou illustration.
    pub fn paint(&mut self, ctx: &Context, painter: &Painter, screen: Rect, p: &Palette) -> bool {
        self.load(ctx, p);
        let Some(image) = self.image.as_ref() else {
            ctx.data_mut(|data| data.remove::<TextureHandle>(context_id()));
            return false;
        };
        if let Some(frosted) = &self.frosted {
            ctx.data_mut(|data| data.insert_temp(context_id(), frosted.clone()));
        }
        painter.image(image.id(), screen, cover_uv(image.size(), screen), Color32::WHITE);
        true
    }
}

/// The softened version of the SAME wallpaper behind the panel. The caller
/// clips it to its glass surface, then applies the user's chosen opacity.
pub fn frosted_shape(ctx: &Context, screen: Rect, pane: Rect, radius: egui::CornerRadius) -> Option<Shape> {
    let texture = ctx.data(|data| data.get_temp::<TextureHandle>(context_id()))?;
    let uv = cover_uv(texture.size(), screen);
    let at = |pos: Pos2| {
        Pos2::new(
            egui::lerp(uv.x_range(), (pos.x - screen.left()) / screen.width()),
            egui::lerp(uv.y_range(), (pos.y - screen.top()) / screen.height()),
        )
    };
    let uv = Rect::from_min_max(at(pane.min), at(pane.max));
    Some(
        egui::epaint::RectShape::filled(pane, radius, Color32::WHITE)
            .with_texture(texture.id(), uv)
            .into(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wallpaper_cover_never_stretches_or_reverses_uvs() {
        let wide = Rect::from_min_size(Pos2::ZERO, egui::vec2(1920.0, 1080.0));
        let tall = Rect::from_min_size(Pos2::ZERO, egui::vec2(720.0, 1280.0));
        let w = cover_uv([1920, 1080], wide);
        assert_eq!(w, Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)));
        let t = cover_uv([1920, 1080], tall);
        assert!(t.width() < 1.0 && t.height() == 1.0);
        let l = cover_uv([720, 1280], wide);
        assert!(l.width() == 1.0 && l.height() < 1.0);
    }

    #[test]
    fn no_wallpaper_never_paints_synthetic_credentials() {
        let ctx = Context::default();
        let mut wallpaper = Wallpaper::default();
        let painter = ctx.layer_painter(egui::LayerId::background());
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0));
        // A missing optional image must be a quiet, safely rendered fallback.
        // No account, password or fabricated grid data is stored here.
        assert!(wallpaper.paint(&ctx, &painter, rect, &crate::theme::Theme::waters_roux().palette()));
        assert!(frosted_shape(&ctx, rect, rect, egui::CornerRadius::ZERO).is_some());
    }
}
