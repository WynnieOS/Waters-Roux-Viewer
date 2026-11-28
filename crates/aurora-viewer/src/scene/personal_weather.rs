//! Waters Roux personal-only precipitation in the existing world particle pass.
//!
//! This is a bounded, camera-local first pass, not the full Aurora GPU wet-ground,
//! accumulated snow, footprints or aurora-sky renderer. Those effects still require
//! their own integration before the owner can approve a complete viewer.
//! It never alters Second Life objects, shared environment or simulator state.

use crate::weather::{WeatherQuality, WeatherSettings};
use aurora_render::{ParticleInstance, particle_flags};
use glam::{Vec2, Vec3};

const HEIGHT: f32 = 26.0;
const HALF_WIDTH: f32 = 24.0;

/// Stable hash of a particle index, with different per-axis salts.
fn unit(index: u32, salt: u32) -> f32 {
    let mut x = index.wrapping_mul(0x9E37_79B9) ^ salt;
    x ^= x >> 16;
    x = x.wrapping_mul(0x7FEB_352D);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846C_A68B);
    x ^= x >> 16;
    (x as f64 / u32::MAX as f64) as f32
}

/// Add depth-tested world-space drops/flakes following the local camera.
/// Limited quality budgets protect the renderer from unbounded instancing.
pub fn append(out: &mut Vec<ParticleInstance>, weather: &WeatherSettings, camera: Vec3, elapsed: f32, wind: Vec2) {
    let weather = weather.in_effect();
    if !camera.is_finite() || !elapsed.is_finite() || !wind.is_finite() {
        return;
    }
    let fall = weather.fall().clamp(0.0, 1.0);
    if !weather.enabled || !weather.precipitation_on || !fall.is_finite() || fall <= 0.001 {
        return;
    }
    let snow = weather.snowing();
    let budget = match weather.quality {
        WeatherQuality::Low => 200,
        WeatherQuality::Medium => 450,
        WeatherQuality::High => 800,
    };
    let n = (budget as f32 * fall).round() as u32;
    let speed = if snow { 1.7 } else { 17.0 };
    let size = if snow {
        0.12 * weather.flake_scale()
    } else {
        0.022
    };
    if !size.is_finite() {
        return;
    }
    let axis = if snow {
        Vec3::ZERO
    } else {
        Vec3::new(wind.x * 0.02, wind.y * 0.02, -1.25)
    };
    for i in 0..n {
        let x = (unit(i, 0xA162_8A1B) * 2.0 - 1.0) * HALF_WIDTH;
        let y = (unit(i, 0x51D3_2CAB) * 2.0 - 1.0) * HALF_WIDTH;
        // Wrap rather than repeatedly spawning and allocating particles.
        // A moving camera never drags a trail of stale particles behind it.
        let z = ((unit(i, 0xA90E_1029) * HEIGHT - elapsed * speed).rem_euclid(HEIGHT)) - 4.0;
        let sway = if snow { (elapsed * 0.5 + i as f32).sin() * 0.35 } else { 0.0 };
        let pos = camera + Vec3::new(x + wind.x * 0.03 + sway, y + wind.y * 0.03, z);
        if pos.distance_squared(camera) < 2.0 {
            continue;
        }
        out.push(ParticleInstance {
            pos: pos.to_array(),
            texture: 0,
            size: if snow { [size, size] } else { [size, axis.length()] },
            flags: if snow {
                particle_flags::EMISSIVE
            } else {
                particle_flags::EMISSIVE | particle_flags::AXIS
            },
            color: if snow { [234, 243, 255, 230] } else { [176, 201, 232, 156] },
            axis: axis.to_array(),
            glow: 0.0,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_weather_draws_nothing() {
        let mut out = Vec::new();
        append(&mut out, &WeatherSettings::default(), Vec3::ZERO, 2.0, Vec2::ZERO);
        assert!(out.is_empty());
    }

    #[test]
    fn raw_saved_switches_and_invalid_inputs_do_not_emit_particles() {
        let mut w = WeatherSettings {
            enabled: true,
            precipitation_on: false,
            touched: true,
            ..Default::default()
        };
        let mut out = Vec::new();
        append(&mut out, &w, Vec3::ZERO, 2.0, Vec2::ZERO);
        assert!(out.is_empty());
        w.precipitation_on = true;
        w.enabled = false;
        append(&mut out, &w, Vec3::ZERO, 2.0, Vec2::ZERO);
        assert!(out.is_empty());
        w.enabled = true;
        append(&mut out, &w, Vec3::ZERO, 2.0, Vec2::new(f32::NAN, 0.0));
        assert!(out.is_empty());
        w.amount = f32::NAN;
        append(&mut out, &w, Vec3::ZERO, 2.0, Vec2::ZERO);
        assert!(out.is_empty());
    }

    #[test]
    fn rain_and_snow_are_real_depth_particle_instances() {
        let mut w = WeatherSettings {
            enabled: true,
            precipitation_on: true,
            amount: 0.6,
            ..Default::default()
        };
        let mut out = Vec::new();
        let eye = Vec3::new(120.0, 130.0, 30.0);
        append(&mut out, &w, eye, 12.0, Vec2::ZERO);
        assert!(out.len() > 100 && out.len() <= 800);
        assert!(out.iter().all(|p| p.pos.iter().all(|v| v.is_finite())));
        assert!(out.iter().all(|p| p.flags & particle_flags::AXIS != 0));
        out.clear();
        w.precipitation = crate::weather::Precipitation::Snow;
        append(&mut out, &w, eye, 13.0, Vec2::ZERO);
        assert!(out.len() > 100);
        assert!(out.iter().all(|p| p.flags & particle_flags::AXIS == 0));
    }

    #[test]
    fn low_quality_and_disabled_switch_reduce_work() {
        let mut w = WeatherSettings {
            enabled: true,
            precipitation_on: true,
            ..Default::default()
        };
        let mut out = Vec::new();
        append(&mut out, &w, Vec3::ZERO, 5.0, Vec2::ZERO);
        let high = out.len();
        w.quality = WeatherQuality::Low;
        out.clear();
        append(&mut out, &w, Vec3::ZERO, 5.0, Vec2::ZERO);
        assert!(out.len() < high);
        w.precipitation_on = false;
        out.clear();
        append(&mut out, &w.in_effect(), Vec3::ZERO, 5.0, Vec2::ZERO);
        assert!(out.is_empty());
    }
}
