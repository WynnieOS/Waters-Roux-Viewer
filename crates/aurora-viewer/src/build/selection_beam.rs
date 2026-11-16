//! Visible, local-only right-hand selection trail for the Build/Edit tool.
//! This is the familiar selection beam, not an object particle emitter.
//! It follows the avatar's *posed* wrist and the actual selected object,
//! without creating simulator particles or transmitting any new effects.

use super::{BuildSettings, BuildTool, geom::Cam};
use crate::{scene::Scene, world::World};
use egui::{Color32, Painter, Pos2, Stroke};
use glam::Vec3;
use std::time::Instant;

const MAX_BEAM_METRES: f32 = 256.0;
const MIN_BEAM_METRES: f32 = 0.08;
const MAX_DOTS: usize = 40;

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

    let count = (span / 14.0).round() as usize;
    let count = count.clamp(4, MAX_DOTS);
    let time = if s.selection_beam_animated {
        ctx.input(|i| i.time) as f32
    } else {
        0.0
    };
    for (i, t) in particle_positions(count, time, s.selection_beam_animated).enumerate() {
        let pos: Pos2 = a.lerp(b, t);
        let pulse = if s.selection_beam_animated {
            0.55 + 0.45 * (time * 6.0 + i as f32 * 1.7).sin().abs()
        } else {
            0.8
        };
        let color = Color32::from_rgba_unmultiplied(r, g, blue, ((alpha as f32) * pulse) as u8);
        painter.circle_filled(pos, 1.2 + 1.4 * pulse, color);
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
