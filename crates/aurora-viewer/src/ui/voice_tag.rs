//! Speaking outline integrated into avatar name tags.
//!
//! Availability and speaking follow LLVOAvatar::idleUpdateVoiceVisualizer and
//! LLWebRTCVoiceClient::predUpdateOwnVolume (originally LGPL 2.1). The requested
//! presentation replaces the dot / waves with a level-driven outline; listening
//! stays neutral and avatars outside voice get a filled red microphone slash.

use crate::theme::Palette;
use egui::{Color32, Rect, Stroke, StrokeKind};
use std::collections::HashMap;
use uuid::Uuid;

/// Time constants in seconds: react quickly, then fade between syllables.
const ATTACK: f32 = 0.035;
const RELEASE: f32 = 0.10;
pub(super) const LEVEL_MIN: f32 = aurora_voice::SPEAKING_AUDIO_LEVEL;
const LEVEL_AMBER: f32 = 0.48;
// WebRTC's participant level peaks at ~0.632 for full-scale PCM, so the
// legacy visualizer's 0.7 red threshold is unreachable locally. Red at
// ~0.58 corresponds to -10 dBFS, the microphone meter's overdrive point.
pub(super) const LEVEL_MAX: f32 = 0.58;
/// Keep the previous voice tag width across speaking, listening and offline states.
pub const TAG_EXTRA_WIDTH: f32 = 30.0;
pub const ICON_SIZE: f32 = 14.0;
pub const ICON_WIDTH: f32 = ICON_SIZE + 4.0;

struct Envelope {
    opacity: f32,
    level: f32,
    seen: f64,
}

impl Envelope {
    fn new(now: f64) -> Self {
        Self {
            opacity: 0.0,
            level: 0.0,
            seen: now,
        }
    }

    fn update(&mut self, level: f32, speaking: bool, now: f64) {
        let dt = (now - self.seen).max(0.0) as f32;
        self.seen = now;
        let target = if speaking { 1.0 } else { 0.0 };
        let tau = if speaking { ATTACK } else { RELEASE };
        self.opacity += (target - self.opacity) * (1.0 - (-dt / tau).exp());
        if speaking {
            let level = if level.is_finite() { level.clamp(0.0, 1.0) } else { 0.0 };
            self.level += (level - self.level) * (1.0 - (-dt / ATTACK).exp());
        }
        // Preserve the last speech color during release.
        if self.opacity < 0.01 {
            self.opacity = 0.0;
            self.level = 0.0;
        }
    }
}

#[derive(Default)]
pub struct VoiceTags {
    envelopes: HashMap<Uuid, Envelope>,
}

/// Idle microphone, not the speaking outline; preserve the skin palette when
/// no resident-specific color is selected.
pub(super) fn idle_color(p: &Palette, options: &crate::settings::ColorSettings) -> Color32 {
    options.voice_idle.map(crate::ui::colors::c).unwrap_or(p.success)
}

/// No voice connection/unavailable. This is not an indication that an
/// otherwise connected resident deliberately muted their microphone.
pub(super) fn unavailable_color(p: &Palette, options: &crate::settings::ColorSettings) -> Color32 {
    options.voice_unavailable.map(crate::ui::colors::c).unwrap_or(p.danger)
}

/// Green → amber → red, calibrated for WebRTC's actual participant range.
pub(super) fn level_color(p: &Palette, level: f32, options: &crate::settings::ColorSettings) -> Color32 {
    let color = |value: Option<crate::settings::Rgba>, fallback: Color32| value.map(crate::ui::colors::c).unwrap_or(fallback);
    let quiet = color(options.voice_quiet, p.success);
    let medium = color(options.voice_medium, p.amber);
    let loud = color(options.voice_loud, p.danger);
    let lerp = |a: Color32, b: Color32, t: f32| {
        let t = t.clamp(0.0, 1.0);
        let mix = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
        Color32::from_rgb(mix(a.r(), b.r()), mix(a.g(), b.g()), mix(a.b(), b.b()))
    };
    if level < LEVEL_AMBER {
        lerp(quiet, medium, (level - LEVEL_MIN) / (LEVEL_AMBER - LEVEL_MIN))
    } else {
        lerp(medium, loud, (level - LEVEL_AMBER) / (LEVEL_MAX - LEVEL_AMBER))
    }
}

impl VoiceTags {
    /// Draw outside the tag so the thicker outline leaves its text unobstructed.
    pub fn paint(&mut self, painter: &egui::Painter, p: &Palette, id: Uuid, rect: Rect, voice: Option<(f32, bool)>, now: f64, fade: f32) {
        let Some((level, speaking)) = voice else {
            self.envelopes.remove(&id);
            return;
        };
        let e = self.envelopes.entry(id).or_insert_with(|| {
            // The first speaking frame must already be visible.
            Envelope::new(now - f64::from(ATTACK))
        });
        e.update(level, speaking, now);
        if e.opacity > 0.0 {
            let volume = ((e.level - LEVEL_MIN) / (LEVEL_MAX - LEVEL_MIN)).clamp(0.0, 1.0);
            // Keep speech readable at a distance and against bright scene backgrounds.
            let opacity = e.opacity * fade.sqrt();
            let width = 3.0 + volume * 1.5;
            let color = level_color(p, e.level, &crate::ui::colors::get()).gamma_multiply(opacity);
            painter.rect_stroke(
                rect,
                3.0,
                Stroke::new(width + 2.0, p.bar.gamma_multiply(opacity)),
                StrokeKind::Outside,
            );
            painter.rect_stroke(rect, 3.0, Stroke::new(width, color), StrokeKind::Outside);
        }
    }

    /// Forget avatars that have left the screen.
    pub fn prune(&mut self, now: f64) {
        self.envelopes.retain(|_, e| now - e.seen < 5.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loud_pcm_reaches_red_without_clipping() {
        let p = crate::theme::Theme::default().palette();
        // A 0.5-amplitude sine has RMS ~0.35: the same signal exercised by
        // aurora_voice::transmit's encoder test, on Firestorm's 16-bit scale.
        let level = 0.18 + 0.1 * (0.35_f32 * 32_768.0).log10();
        let mut e = Envelope::new(0.0);
        e.update(level, true, 0.3);
        let defaults = crate::settings::ColorSettings::default();
        assert_eq!(level_color(&p, e.level, &defaults), p.danger);
        assert_eq!(level_color(&p, 0.30, &defaults), p.success);
        assert_ne!(level_color(&p, 0.48, &defaults), p.danger);
    }

    #[test]
    fn configurable_voice_colors_preserve_theme_defaults() {
        let p = crate::theme::Theme::default().palette();
        let defaults = crate::settings::ColorSettings::default();
        assert_eq!(level_color(&p, LEVEL_MIN, &defaults), p.success);
        assert_eq!(level_color(&p, LEVEL_AMBER, &defaults), p.amber);
        assert_eq!(level_color(&p, LEVEL_MAX, &defaults), p.danger);

        // Older saved files omit the new keys, and continue to use the theme.
        let from_old: crate::settings::ColorSettings = serde_json::from_str("{}").expect("older color preferences");
        assert_eq!(from_old.voice_quiet, None);
        assert_eq!(from_old.voice_medium, None);
        assert_eq!(from_old.voice_loud, None);

        let mut selected = defaults;
        selected.voice_quiet = Some([255, 0, 128, 255]);
        selected.voice_medium = Some([0, 255, 255, 255]);
        selected.voice_loud = Some([128, 0, 255, 255]);
        assert_eq!(level_color(&p, LEVEL_MIN, &selected), crate::ui::colors::c([255, 0, 128, 255]));
        assert_eq!(level_color(&p, LEVEL_AMBER, &selected), crate::ui::colors::c([0, 255, 255, 255]));
        assert_eq!(level_color(&p, LEVEL_MAX, &selected), crate::ui::colors::c([128, 0, 255, 255]));
    }

    #[test]
    fn voice_idle_and_unavailable_colors_roundtrip_without_reclassifying_muted_users() {
        let p = crate::theme::Theme::default().palette();
        let old: crate::settings::ColorSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(idle_color(&p, &old), p.success);
        assert_eq!(unavailable_color(&p, &old), p.danger);
        assert_eq!(level_color(&p, LEVEL_MIN, &old), p.success);

        let mut selected = old;
        selected.voice_idle = Some([18, 102, 170, 255]);
        selected.voice_unavailable = Some([180, 40, 90, 255]);
        let restored: crate::settings::ColorSettings = serde_json::from_str(&serde_json::to_string(&selected).unwrap()).unwrap();
        assert_eq!(idle_color(&p, &restored), crate::ui::colors::c([18, 102, 170, 255]));
        assert_eq!(unavailable_color(&p, &restored), crate::ui::colors::c([180, 40, 90, 255]));
        // Speech volume remains independently configurable.
        assert_eq!(level_color(&p, LEVEL_AMBER, &restored), p.amber);
    }

    #[test]
    fn listening_never_activates_outline_even_with_a_stale_level() {
        let mut e = Envelope::new(0.0);
        e.update(0.8, false, 1.0);
        assert_eq!(e.opacity, 0.0);
        assert_eq!(e.level, 0.0);
    }

    #[test]
    fn speech_reacts_quickly_and_fades_with_its_last_level() {
        let mut e = Envelope::new(0.0);
        e.update(0.6, true, 0.08);
        assert!(e.opacity > 0.85);
        assert!(e.level > 0.5);
        let level = e.level;
        e.update(0.0, false, 0.18);
        assert!(e.opacity > 0.0 && e.opacity < 0.4);
        assert_eq!(e.level, level);
        e.update(0.0, false, 0.7);
        assert_eq!(e.opacity, 0.0);
    }

    #[test]
    fn envelope_is_independent_of_frame_rate() {
        let run = |fps: u32| {
            let mut e = Envelope::new(0.0);
            for frame in 1..=fps {
                e.update(0.5, true, frame as f64 / fps as f64);
            }
            e
        };
        let (a, b) = (run(30), run(144));
        assert!((a.opacity - b.opacity).abs() < 0.0001);
        assert!((a.level - b.level).abs() < 0.0001);
    }
}
