//! Optional Waters Roux bayou wallpaper behind both login modes.
//!
//! The owner's original background belongs in `assets/branding/` next to the
//! executable, or in the viewer-specific configuration folder's `branding/`.
//! There is no fabricated grid state, account detail or login form in this art.
//! The existing Aurora procedural sky remains a safe fallback until the
//! approved background-only PNG is installed.
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
    Id::new("waters_roux_login_frosted_wallpaper")
}

fn candidates() -> Vec<PathBuf> {
    let mut paths = vec![crate::settings::config_dir().join("branding").join(FILE)];
    if let Some(dir) = std::env::current_exe().ok().and_then(|exe| exe.parent().map(std::path::Path::to_path_buf)) {
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

impl Wallpaper {
    fn load(&mut self, ctx: &Context) {
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
    }

    /// Returns false until a standalone wallpaper asset has been installed.
    /// The login will continue to draw Aurora's existing procedural fallback.
    pub fn paint(&mut self, ctx: &Context, painter: &Painter, screen: Rect) -> bool {
        self.load(ctx);
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
pub fn frosted_shape(ctx: &Context, screen: Rect) -> Option<Shape> {
    let texture = ctx.data(|data| data.get_temp::<TextureHandle>(context_id()))?;
    Some(Shape::image(texture.id(), screen, cover_uv(texture.size(), screen), Color32::WHITE))
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
        assert!(!wallpaper.paint(&ctx, &painter, rect));
        assert!(frosted_shape(&ctx, rect).is_none());
    }
}
