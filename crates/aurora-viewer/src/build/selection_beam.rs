//! Visible, local-only right-hand selection trail for the Build/Edit tool.
//! This is the familiar selection beam, not an object particle emitter.
//! It follows the avatar's *posed* wrist and the actual selected object,
//! without creating simulator particles or transmitting any new effects.

use super::{BuildSettings, BuildTool, geom::Cam};
use crate::{scene::Scene, world::World};
use egui::{Align2, Color32, FontId, Painter, Pos2, Rect, Stroke, TextureHandle, TextureOptions, Vec2};
use glam::Vec3;
use std::{cell::RefCell, io::Cursor, time::Instant};

const MAX_BEAM_METRES: f32 = 256.0;
const MIN_BEAM_METRES: f32 = 0.08;
const MAX_DOTS: usize = 80;
const MAX_CUSTOM_PNG: u64 = 8 * 1024 * 1024;

// Cache only the resident's chosen local texture, never issue simulator particles.
thread_local! {
    static CUSTOM_BEAM: RefCell<Option<(String, Option<TextureHandle>)>> = const { RefCell::new(None) };
}

fn load_png(ctx: &egui::Context, path: &str) -> Option<TextureHandle> {
    if path.len() > 2048 || !path.to_ascii_lowercase().ends_with(".png") {
        return None;
    }
    if std::fs::metadata(path).ok()?.len() > MAX_CUSTOM_PNG {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    let reader = image::ImageReader::new(Cursor::new(&bytes)).with_guessed_format().ok()?;
    let (w, h) = reader.into_dimensions().ok()?;
    if w == 0 || h == 0 || w > 2048 || h > 2048 {
        return None;
    }
    let mut rgba = image::load_from_memory(&bytes).ok()?.to_rgba8();
    let longest = w.max(h);
    if longest > 256 {
        let scale = 256.0 / longest as f32;
        rgba = image::imageops::resize(
            &rgba,
            (w as f32 * scale).round().max(1.0) as u32,
            (h as f32 * scale).round().max(1.0) as u32,
            image::imageops::FilterType::Lanczos3,
        );
    }
    let (w, h) = rgba.dimensions();
    let image = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], rgba.as_raw());
    Some(ctx.load_texture("waters-roux-selection-beam", image, TextureOptions::LINEAR))
}

/// The imported PNG is optional and affects only the resident's own viewer.
pub fn custom_texture(ctx: &egui::Context, path: &str) -> Option<TextureHandle> {
    if path.is_empty() {
        return None;
    }
    CUSTOM_BEAM.with_borrow_mut(|cache| {
        if cache.as_ref().is_none_or(|(loaded, _)| loaded != path) {
            *cache = Some((path.to_owned(), load_png(ctx, path)));
        }
        cache.as_ref().and_then(|(_, texture)| texture.clone())
    })
}

/// True only for an editable world object; HUDs, avatars, an inactive build
/// tool and an empty selection never draw a beam.
fn ends(build: &BuildTool, world: &World, scene: &Scene, now: Instant) -> Option<(Vec3, Vec3)> {
    if !build.open || build.selection.is_empty() {
        return None;
    }
    let target = build.pointing_target(world, now)?;
    let to = scene.look_target_pos(world, world.agent_id, &target, now)?;
    // This uses the actual posed avatar wrist rather than a screen-space
    // approximation, so it follows the hand during animations and posing.
    let from = scene
        .joint_world(world, &world.agent_id, "mWristRight", now)
        .or_else(|| scene.joint_world(world, &world.agent_id, "mHandRight", now))?;
    let len = from.distance(to);
    (from.is_finite() && to.is_finite() && (MIN_BEAM_METRES..=MAX_BEAM_METRES).contains(&len)).then_some((from, to))
}

fn particle_positions(count: usize, time: f32, animated: bool) -> impl Iterator<Item = f32> {
    (0..count).map(move |i| {
        let fraction = (i as f32 + 0.5) / count.max(1) as f32;
        if animated { (fraction + time * 0.55).fract() } else { fraction }
    })
}

pub fn draw(
    build: &BuildTool,
    world: &World,
    scene: &Scene,
    s: &BuildSettings,
    ctx: &egui::Context,
    painter: &Painter,
    cam: &Cam,
    now: Instant,
) {
    if !s.show_selection_beam || cam.hud.is_some() {
        return;
    }
    let Some((from, to)) = ends(build, world, scene, now) else {
        return;
    };
    let (Some(a), Some(b)) = (cam.project(from), cam.project(to)) else {
        return;
    };
    let span = a.distance(b);
    if !span.is_finite() || span < 3.0 {
        return;
    }
    let [r, g, blue, alpha] = s.selection_beam_color;
    let base = Color32::from_rgba_unmultiplied(r, g, blue, alpha);
    let line = Color32::from_rgba_unmultiplied(r, g, blue, alpha / 3);
    painter.line_segment([a, b], Stroke::new(1.2, line));

    let density = s.selection_beam_density.clamp(25, 200) as f32 / 100.0;
    let count = ((span / 14.0) * density).round() as usize;
    let count = count.clamp(2, MAX_DOTS);
    let time = if s.selection_beam_animated {
        ctx.input(|i| i.time) as f32 * s.selection_beam_speed.clamp(0.1, 4.0)
    } else {
        0.0
    };
    let size = if s.selection_beam_size.is_finite() {
        s.selection_beam_size.clamp(2.0, 20.0)
    } else {
        4.0
    };
    let custom = if s.selection_beam_style == "custom" {
        custom_texture(ctx, &s.selection_beam_texture)
    } else {
        None
    };
    for (i, t) in particle_positions(count, time, s.selection_beam_animated).enumerate() {
        let pos: Pos2 = a.lerp(b, t);
        let pulse = if s.selection_beam_animated {
            0.55 + 0.45 * (time * 6.0 + i as f32 * 1.7).sin().abs()
        } else {
            0.8
        };
        let color = Color32::from_rgba_unmultiplied(r, g, blue, ((alpha as f32) * pulse) as u8);
        let radius = size * (0.55 + 0.3 * pulse);
        match s.selection_beam_style.as_str() {
            "sparkles" => {
                let arm = radius * 1.5;
                painter.line_segment([pos - Vec2::new(arm, 0.0), pos + Vec2::new(arm, 0.0)], Stroke::new(1.3, color));
                painter.line_segment([pos - Vec2::new(0.0, arm), pos + Vec2::new(0.0, arm)], Stroke::new(1.3, color));
            }
            "stars" => {
                painter.text(pos, Align2::CENTER_CENTER, "★", FontId::proportional(radius * 2.4), color);
            }
            "hearts" => {
                painter.text(pos, Align2::CENTER_CENTER, "♥", FontId::proportional(radius * 2.4), color);
            }
            "custom" if custom.is_some() => {
                if let Some(texture) = &custom {
                    let rect = Rect::from_center_size(pos, Vec2::splat(radius * 3.0));
                    painter.image(texture.id(), rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), color);
                }
            }
            _ => {
                painter.circle_filled(pos, radius * 0.65, color);
            }
        }
    }
    // Bright but restrained end marker, so the selection target is visible
    // even when the selected mesh has a pale or complex texture.
    painter.circle_stroke(b, 5.0, Stroke::new(1.5, base));
    if s.selection_beam_animated {
        ctx.request_repaint();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_particles_remain_evenly_spaced() {
        let a: Vec<_> = particle_positions(4, 900.0, false).collect();
        assert_eq!(a, [0.125, 0.375, 0.625, 0.875]);
    }

    #[test]
    fn animated_particles_wrap_and_do_not_overshoot() {
        for t in particle_positions(40, 1234.5, true) {
            assert!((0.0..1.0).contains(&t));
        }
    }

    #[test]
    fn selection_beam_is_on_by_default_and_has_finite_color() {
        let s = BuildSettings::default();
        assert!(s.show_selection_beam);
        assert!(s.selection_beam_animated);
        assert!(s.selection_beam_color[3] > 0);
    }
}
