//! Local-only login and loading artwork. Decode, resize and frost work stays
//! off the frame thread; each independent slot keeps a bounded GPU texture.
//! No personal image is uploaded, and these textures never replace the
//! executable, About-page or system-notification identity.
use crate::branding::{Accent, Appearance, ImageChoice, ImageSettings, Slot, Tint};
use crate::theme::Palette;
use egui::{Context, CornerRadius, Id, Painter, Pos2, Rect, Shape, TextureHandle, TextureOptions, Vec2};
use std::sync::{Arc, Mutex, mpsc};

const SLOTS: [Slot; 4] = [Slot::LoginBackground, Slot::LoginLogo, Slot::LoadingBackground, Slot::LoadingLogo];

fn state_id() -> Id {
    Id::unique("waters_roux_branding_runtime")
}

#[derive(Clone, PartialEq)]
struct AssetKey {
    choice: ImageChoice,
    blur: f32,
}

struct Decoded {
    native_size: [u32; 2],
    sharp: image::RgbaImage,
    soft: Option<image::RgbaImage>,
}

struct Textures {
    native_size: [u32; 2],
    sharp: TextureHandle,
    soft: Option<TextureHandle>,
}

#[derive(Default)]
struct Entry {
    key: Option<AssetKey>,
    pending: Option<(AssetKey, mpsc::Receiver<Option<Decoded>>)>,
    textures: Option<Textures>,
}

#[derive(Default)]
struct Runtime {
    saved: Option<Appearance>,
    preview: Option<Appearance>,
    entries: [Entry; SLOTS.len() * 2],
    neutral: Option<([egui::Color32; 3], TextureHandle)>,
}

fn state(ctx: &Context) -> Arc<Mutex<Runtime>> {
    ctx.data_mut(|data| {
        if let Some(state) = data.get_temp::<Arc<Mutex<Runtime>>>(state_id()) {
            return state;
        }
        let state = Arc::new(Mutex::new(Runtime::default()));
        data.insert_temp(state_id(), state.clone());
        state
    })
}

/// Effective settings for this frame: an explicit editor preview, otherwise
/// the safely loaded local preferences. No file is read again per frame.
pub fn current(ctx: &Context) -> Appearance {
    let state = state(ctx);
    let Ok(mut state) = state.lock() else {
        return Appearance::default();
    };
    if let Some(preview) = &state.preview {
        return preview.clone();
    }
    state
        .saved
        .get_or_insert_with(|| {
            let mut saved = crate::branding::load();
            if !crate::branding::has_saved()
                && let Some(alpha) = crate::interface_colors::current().glass_alpha
            {
                saved.glass.opacity = (f32::from(alpha) * 100.0 / 255.0).round().clamp(10.0, 100.0) as u8;
            }
            saved
        })
        .clone()
}

/// A draft affects only this process until Apply. Clearing it immediately
/// restores the last committed configuration, including all four choices.
pub fn set_preview(ctx: &Context, appearance: Option<Appearance>) {
    if let Ok(mut state) = state(ctx).lock() {
        state.preview = appearance;
    }
    ctx.request_repaint();
}

/// Call after a successful atomic save. Texture keys track image choice and
/// blur only, so moving/scaling an image never starts another decode job.
pub fn reload(ctx: &Context) {
    if let Ok(mut state) = state(ctx).lock() {
        state.saved = None;
    }
    ctx.request_repaint();
}

/// Preview opacity is immediate; outside the editor the existing Appearance
/// opacity preference remains authoritative, preserving Beta 26 settings.
/// High Contrast continues to supply its opaque accessible palette intact.
pub fn palette(ctx: &Context, base: &Palette) -> Palette {
    if base.high_contrast {
        return *base;
    }
    let appearance = current(ctx);
    let previewing = state(ctx).lock().ok().is_some_and(|state| state.preview.is_some());
    let mut p = *base;
    p.panel_alpha = if previewing {
        ((f32::from(appearance.glass.opacity) * 255.0 / 100.0).round() as u8).clamp(26, 255)
    } else {
        crate::interface_colors::current()
            .glass_alpha
            .unwrap_or_else(|| (f32::from(appearance.glass.opacity) * 255.0 / 100.0).round() as u8)
    };
    p.panel = match appearance.glass.tint {
        Tint::Mist => base.panel,
        Tint::Blush => base.rose,
        Tint::Cloud => base.field,
    };
    p.violet = match appearance.glass.accent {
        Accent::Blush => base.violet,
        Accent::Blue => base.teal,
        Accent::Gold => base.amber,
    };
    if matches!(appearance.glass.accent, Accent::Gold) {
        p.on_accent = base.ink;
    }
    if crate::interface_colors::contrast(p.on_accent, p.violet) < 4.5 {
        if crate::interface_colors::contrast(base.ink, p.violet) >= 4.5 {
            p.on_accent = base.ink;
        } else {
            // Decorative powder blue stays unchanged. Only the actionable
            // face darkens enough for its label to meet normal-text contrast.
            for step in 1..=16 {
                let face = p.violet.lerp_to_gamma(base.ink, step as f32 / 16.0);
                if crate::interface_colors::contrast(p.on_accent, face) >= 4.5 {
                    p.violet = face;
                    break;
                }
            }
        }
    }
    p
}

fn slot_index(slot: Slot) -> usize {
    match slot {
        Slot::LoginBackground => 0,
        Slot::LoginLogo => 1,
        Slot::LoadingBackground => 2,
        Slot::LoadingLogo => 3,
    }
}

fn is_background(slot: Slot) -> bool {
    matches!(slot, Slot::LoginBackground | Slot::LoadingBackground)
}

fn texture(ctx: &Context, label: &str, pixels: &image::RgbaImage) -> TextureHandle {
    ctx.load_texture(
        label,
        egui::ColorImage::from_rgba_unmultiplied([pixels.width() as usize, pixels.height() as usize], pixels.as_raw()),
        TextureOptions::LINEAR,
    )
}

fn prepare(ctx: &Context, appearance: &Appearance, slot: Slot, preview: bool) -> Option<(TextureHandle, Option<TextureHandle>, [u32; 2])> {
    let settings = appearance.image(slot).clone();
    let key = AssetKey {
        choice: settings.choice.clone(),
        blur: if is_background(slot) {
            appearance.glass.blur.clamp(0.0, 24.0)
        } else {
            0.0
        },
    };
    let state = state(ctx);
    let Ok(mut state) = state.lock() else { return None };
    let entry = &mut state.entries[slot_index(slot) + if preview { SLOTS.len() } else { 0 }];
    // At most one worker per slot. Fast slider changes coalesce into the
    // newest request instead of spawning unbounded blur/decode threads.
    if let Some((pending_key, receiver)) = &entry.pending {
        match receiver.try_recv() {
            Ok(decoded) => {
                let ready_key = pending_key.clone();
                if ready_key == key {
                    entry.textures = decoded.map(|decoded| Textures {
                        native_size: decoded.native_size,
                        sharp: texture(ctx, "waters_roux_artwork", &decoded.sharp),
                        soft: decoded
                            .soft
                            .as_ref()
                            .map(|image| texture(ctx, "waters_roux_frosted_artwork", image)),
                    });
                    entry.key = Some(ready_key);
                }
                entry.pending = None;
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                entry.key = Some(pending_key.clone());
                entry.textures = None;
                entry.pending = None;
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
    }
    if entry.key.as_ref() != Some(&key) && entry.pending.is_none() {
        entry.textures = None;
        let (sender, receiver) = mpsc::channel();
        entry.pending = Some((key.clone(), receiver));
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let decoded = crate::branding::load_image(&settings, slot).map(|image| {
                let native_size = [image.width(), image.height()];
                let bound = if is_background(slot) { (2560, 1600) } else { (1024, 1024) };
                let sharp = image.thumbnail(bound.0, bound.1).to_rgba8();
                let soft = if is_background(slot) {
                    let small = image.thumbnail(800, 600).to_rgba8();
                    Some(if key.blur > 0.0 {
                        image::imageops::blur(&small, key.blur)
                    } else {
                        small
                    })
                } else {
                    None
                };
                Decoded { native_size, sharp, soft }
            });
            let _ = sender.send(decoded);
            ctx.request_repaint();
        });
    }
    entry.textures.as_ref().map(|t| (t.sharp.clone(), t.soft.clone(), t.native_size))
}

/// A quiet neutral background when official artwork is missing. It is
/// deliberately not presented as the approved New Orleans image.
fn neutral(ctx: &Context, p: &Palette) -> TextureHandle {
    let colors = [p.navy, p.panel, p.rose];
    let state = state(ctx);
    if let Ok(mut state) = state.lock() {
        if let Some((have, texture)) = &state.neutral
            && have == &colors
        {
            return texture.clone();
        }
        let pixels = image::RgbaImage::from_fn(64, 64, |_, y| {
            let v = y as f32 / 63.0;
            let c = if v < 0.55 {
                p.navy.lerp_to_gamma(p.panel, v / 0.55)
            } else {
                p.panel.lerp_to_gamma(p.rose, (v - 0.55) / 0.45 * 0.45)
            };
            image::Rgba([c.r(), c.g(), c.b(), 255])
        });
        let texture = texture(ctx, "waters_roux_missing_official_background", &pixels);
        state.neutral = Some((colors, texture.clone()));
        texture
    } else {
        ctx.load_texture(
            "waters_roux_neutral",
            egui::ColorImage::filled([1, 1], p.panel),
            TextureOptions::LINEAR,
        )
    }
}

fn background_slot(loading: bool) -> Slot {
    if loading { Slot::LoadingBackground } else { Slot::LoginBackground }
}

fn logo_slot(loading: bool) -> Slot {
    if loading { Slot::LoadingLogo } else { Slot::LoginLogo }
}

fn picture(painter: &Painter, texture: &TextureHandle, destination: Rect, uv: Rect, alpha: f32) {
    painter.image(
        texture.id(),
        destination,
        uv,
        super::widgets::PICTURE_TINT.gamma_multiply(alpha.clamp(0.0, 1.0)),
    );
}

fn full_uv() -> Rect {
    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0))
}

fn paint_background_for(
    ctx: &Context,
    painter: &Painter,
    screen: Rect,
    p: &Palette,
    appearance: &Appearance,
    loading: bool,
    alpha: f32,
    preview: bool,
) -> bool {
    let slot = background_slot(loading);
    let ready = prepare(ctx, appearance, slot, preview);
    // Loading keeps its real last-view fallback when no chosen/official
    // artwork is available. The neutral login fallback must not cover it.
    if loading && ready.is_none() {
        return false;
    }
    let neutral = neutral(ctx, p);
    if let Some((texture, _, native_size)) = ready {
        let (destination, uv) = crate::branding::placement(appearance.image(slot), native_size, screen);
        // Paint letterboxing only outside the image: painting two translucent
        // layers on top of one another would make teleport fades too opaque.
        let shown = destination.intersect(screen);
        for area in [
            Rect::from_min_max(screen.min, Pos2::new(screen.right(), shown.top())),
            Rect::from_min_max(Pos2::new(screen.left(), shown.bottom()), screen.max),
            Rect::from_min_max(Pos2::new(screen.left(), shown.top()), shown.left_bottom()),
            Rect::from_min_max(shown.right_top(), Pos2::new(screen.right(), shown.bottom())),
        ] {
            if area.is_positive() {
                picture(
                    &painter.with_clip_rect(area.intersect(painter.clip_rect())),
                    &neutral,
                    screen,
                    full_uv(),
                    alpha,
                );
            }
        }
        picture(
            &painter.with_clip_rect(screen.intersect(painter.clip_rect())),
            &texture,
            destination,
            uv,
            alpha,
        );
    } else {
        picture(painter, &neutral, screen, full_uv(), alpha);
    }
    true
}

/// Login and loading always have independent image and transform settings.
/// The existing loading-progress and operational notices remain the caller's.
pub fn paint_background(ctx: &Context, painter: &Painter, screen: Rect, p: &Palette, loading: bool) -> bool {
    paint_background_alpha(ctx, painter, screen, p, loading, 1.0)
}

/// Loading/teleport fades apply to the image as well as the progress text.
pub fn paint_background_alpha(ctx: &Context, painter: &Painter, screen: Rect, p: &Palette, loading: bool, alpha: f32) -> bool {
    paint_background_for(ctx, painter, screen, p, &current(ctx), loading, alpha, false)
}

fn cropped_shape(texture: &TextureHandle, destination: Rect, uv: Rect, pane: Rect, radius: CornerRadius) -> Option<Shape> {
    let crop = destination.intersect(pane);
    if !crop.is_positive() {
        return None;
    }
    let at = |pos: Pos2| {
        Pos2::new(
            egui::lerp(uv.x_range(), (pos.x - destination.left()) / destination.width()),
            egui::lerp(uv.y_range(), (pos.y - destination.top()) / destination.height()),
        )
    };
    Some(
        egui::epaint::RectShape::filled(crop, radius, super::widgets::PICTURE_TINT)
            .with_texture(texture.id(), Rect::from_min_max(at(crop.min), at(crop.max)))
            .into(),
    )
}

/// The same background pixels, transform and crop behind the card, softened
/// off-thread. The caller applies its tint once over these rounded textures.
pub fn frosted_shape(ctx: &Context, screen: Rect, pane: Rect, radius: CornerRadius) -> Option<Shape> {
    frosted_shape_for(ctx, &current(ctx), screen, pane, radius, false)
}

fn frosted_shape_for(
    ctx: &Context,
    appearance: &Appearance,
    screen: Rect,
    pane: Rect,
    radius: CornerRadius,
    preview: bool,
) -> Option<Shape> {
    let existing = {
        let state = state(ctx);
        state
            .lock()
            .ok()
            .and_then(|state| state.neutral.as_ref().map(|(_, texture)| texture.clone()))
    };
    let neutral = existing.unwrap_or_else(|| neutral(ctx, &crate::theme::Theme::waters_roux().palette()));
    let mut shapes = vec![cropped_shape(&neutral, screen, full_uv(), pane, radius)?];
    if let Some((_, Some(soft), native_size)) = prepare(ctx, appearance, Slot::LoginBackground, preview) {
        let (destination, uv) = crate::branding::placement(&appearance.login.background, native_size, screen);
        if let Some(shape) = cropped_shape(&soft, destination, uv, pane, radius) {
            shapes.push(shape);
        }
    }
    Some(Shape::Vec(shapes))
}

/// Custom screen logos are confined to presentation; the application-owned
/// fallback remains the approved official emblem and is never overwritten.
pub fn logo(ctx: &Context, loading: bool, fallback: Option<&TextureHandle>) -> Option<TextureHandle> {
    let appearance = current(ctx);
    let slot = logo_slot(loading);
    if matches!(appearance.image(slot).choice, ImageChoice::Hidden) {
        return None;
    }
    prepare(ctx, &appearance, slot, false)
        .map(|(sharp, _, _)| sharp)
        .or_else(|| fallback.cloned())
}

fn logo_rect_for(settings: &ImageSettings, bounds: Rect, texture: &TextureHandle) -> Rect {
    let aspect = texture.aspect_ratio().max(0.001);
    let width = (bounds.width() * settings.size * settings.scale).clamp(1.0, bounds.width().max(1.0));
    let height = (width / aspect).min(bounds.height().max(1.0));
    let width = (height * aspect).min(bounds.width().max(1.0));
    let size = Vec2::new(width, width / aspect);
    let x = bounds.left() + (bounds.width() - size.x).max(0.0) * settings.x.clamp(0.0, 1.0);
    let y = bounds.top() + (bounds.height() - size.y).max(0.0) * settings.y.clamp(0.0, 1.0);
    Rect::from_min_size(Pos2::new(x, y), size)
}

/// Use the screen rect or a reserved header rect. Both placement and sizing
/// stay within those bounds, preserving the source logo's aspect ratio.
pub fn logo_rect(ctx: &Context, loading: bool, bounds: Rect, texture: &TextureHandle) -> Rect {
    let appearance = current(ctx);
    logo_rect_for(appearance.image(logo_slot(loading)), bounds, texture)
}

pub fn paint_logo(ctx: &Context, painter: &Painter, bounds: Rect, loading: bool, fallback: Option<&TextureHandle>, alpha: f32) -> bool {
    let Some(texture) = logo(ctx, loading, fallback) else {
        return false;
    };
    let rect = logo_rect(ctx, loading, bounds, &texture);
    painter.image(
        texture.id(),
        rect,
        full_uv(),
        super::widgets::PICTURE_TINT.gamma_multiply(alpha.clamp(0.0, 1.0)),
    );
    true
}

/// The editor's image preview uses exactly the runtime textures and placement
/// math; the full login screen also previews the draft before Apply.
pub fn paint_preview(ui: &mut egui::Ui, rect: Rect, p: &Palette, appearance: &Appearance, loading: bool) {
    let ctx = ui.ctx();
    let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
    if !paint_background_for(ctx, &painter, rect, p, appearance, loading, 1.0, true) {
        picture(&painter, &neutral(ctx, p), rect, full_uv(), 1.0);
    }
    let slot = logo_slot(loading);
    if !matches!(appearance.image(slot).choice, ImageChoice::Hidden)
        && let Some((texture, _, _)) = prepare(ctx, appearance, slot, true)
    {
        let logo_rect = logo_rect_for(appearance.image(slot), rect, &texture);
        picture(&painter, &texture, logo_rect, full_uv(), 1.0);
    }
    let card = Rect::from_min_size(rect.min + rect.size() * Vec2::new(0.22, 0.47), rect.size() * Vec2::new(0.56, 0.40));
    let radius = ctx.global_style().visuals.window_corner_radius;
    let alpha = if p.high_contrast {
        255
    } else {
        (f32::from(appearance.glass.opacity) * 255.0 / 100.0).round() as u8
    };
    if !p.high_contrast && alpha < 255 {
        // Use the selected preview screen's own background and blur, not the
        // login cache, so the Loading tab is independently verifiable.
        let background = background_slot(loading);
        if let Some((_, Some(soft), native_size)) = prepare(ctx, appearance, background, true) {
            let (destination, uv) = crate::branding::placement(appearance.image(background), native_size, rect);
            if let Some(shape) = cropped_shape(&soft, destination, uv, card, radius) {
                painter.add(shape);
            }
        }
        let glow = appearance.glass.glow.clamp(0.0, 1.0);
        super::aurora_bg::radial_glow(&painter, card.left_top(), card.width() * 0.7, p.rose, glow * 0.18);
        super::aurora_bg::radial_glow(&painter, card.right_bottom(), card.width() * 0.7, p.indigo_light, glow * 0.18);
    }
    let tint = match appearance.glass.tint {
        Tint::Mist => p.panel,
        Tint::Blush => p.rose,
        Tint::Cloud => p.field,
    };
    let tint = if p.high_contrast { p.panel } else { tint };
    painter.rect_filled(
        card,
        radius,
        egui::Color32::from_rgba_unmultiplied(tint.r(), tint.g(), tint.b(), alpha),
    );
    let edge = if p.high_contrast {
        p.muted_dim
    } else {
        p.field.gamma_multiply(0.4 + appearance.glass.glow * 0.3)
    };
    painter.rect_stroke(card, radius, egui::Stroke::new(1.0, edge), egui::StrokeKind::Inside);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branding_runtime_logo_placement_preserves_aspect_and_bounds() {
        let ctx = Context::default();
        let texture = ctx.load_texture(
            "test-logo",
            egui::ColorImage::filled([300, 120], crate::theme::Theme::waters_roux().palette().panel),
            TextureOptions::LINEAR,
        );
        let mut settings = Appearance::default().login.logo;
        settings.size = 0.8;
        settings.scale = 3.0;
        settings.x = 1.0;
        settings.y = 1.0;
        for size in [Vec2::new(1920.0, 1080.0), Vec2::new(1366.0, 768.0), Vec2::new(420.0, 920.0)] {
            let bounds = Rect::from_min_size(Pos2::ZERO, size);
            let rect = logo_rect_for(&settings, bounds, &texture);
            assert!(bounds.contains_rect(rect));
            assert!((rect.aspect_ratio() - 2.5).abs() < 0.001);
        }
    }

    #[test]
    fn branding_runtime_cancel_restores_all_four_saved_choices() {
        let ctx = Context::default();
        let saved = current(&ctx);
        let mut draft = saved.clone();
        for slot in SLOTS {
            draft.image_mut(slot).choice = ImageChoice::Hidden;
        }
        set_preview(&ctx, Some(draft.clone()));
        assert_eq!(current(&ctx), draft);
        set_preview(&ctx, None);
        assert_eq!(current(&ctx), saved);
    }

    #[test]
    fn branding_runtime_primary_accents_keep_readable_labels() {
        let ctx = Context::default();
        let base = crate::theme::Theme::waters_roux().skin_palette();
        for accent in [Accent::Blush, Accent::Blue, Accent::Gold] {
            let mut appearance = Appearance::default();
            appearance.glass.accent = accent;
            set_preview(&ctx, Some(appearance));
            let shown = palette(&ctx, &base);
            assert!(
                crate::interface_colors::contrast(shown.on_accent, shown.violet) >= 4.5,
                "{accent:?}"
            );
        }
    }

    #[test]
    fn branding_runtime_editor_cache_is_independent_of_the_live_screen() {
        let ctx = Context::default();
        let mut live = Appearance::default();
        live.login.background.choice = ImageChoice::Hidden;
        let draft = Appearance::default();
        prepare(&ctx, &live, Slot::LoginBackground, false);
        prepare(&ctx, &draft, Slot::LoginBackground, true);
        let state = state(&ctx);
        let state = state.lock().expect("runtime cache");
        assert_eq!(
            state.entries[0].pending.as_ref().expect("live request").0.choice,
            ImageChoice::Hidden
        );
        assert_eq!(
            state.entries[SLOTS.len()].pending.as_ref().expect("preview request").0.choice,
            ImageChoice::Official
        );
    }

    #[test]
    fn branding_runtime_hidden_loading_artwork_preserves_the_real_last_view() {
        let ctx = Context::default();
        let mut appearance = Appearance::default();
        appearance.loading.background.choice = ImageChoice::Hidden;
        let painter = ctx.layer_painter(egui::LayerId::background());
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));
        assert!(!paint_background_for(
            &ctx,
            &painter,
            screen,
            &crate::theme::Theme::waters_roux().skin_palette(),
            &appearance,
            true,
            0.5,
            false
        ));
    }
}
