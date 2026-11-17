//! Dedicated creator cinematography panel. This is NOT general Camera Controls:
//! it exposes live camera/lighting render settings and an opt-in native recorder.

use super::widgets::{self, ActionKind, Floater};
use crate::{camera::CinematicTarget, settings::Settings, theme::Palette};
use egui::{Context, Vec2};

pub const WINDOW: &str = "waters_roux_machinima";

/// Match Preferences and PhotoTools: the renderer's shadow switch and
/// quality slider must always stay in sync.
fn set_machinima_shadow_quality(settings: &mut Settings, quality: u8) {
    settings.shadow_quality = quality.min(4);
    settings.shadows = settings.shadow_quality > 0;
}

/// FSR owns world render resolution; match PhotoTools and Graphics Preferences.
fn manual_render_scale_enabled(settings: &Settings) -> bool {
    settings.antialiasing != 7
}

/// The existing renderer disables CAS when HDR is off or FSR supplies RCAS.
fn cas_sharpen_enabled(settings: &Settings) -> bool {
    settings.hdr_emissive && settings.antialiasing != 7
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Action {
    #[default]
    None,
    TogglePov,
    ResetCamera,
    CameraControls,
    PhotoTools,
    Snapshot,
    Panorama,
    ToggleInterface,
    StartRecording,
    StopRecording,
}

/// Return an explicit camera/recording action and whether live settings changed.
pub fn show(
    ctx: &Context,
    p: &Palette,
    open: &mut bool,
    pov: bool,
    recording: bool,
    finalizing: bool,
    elapsed: std::time::Duration,
    settings: &mut Settings,
) -> (Action, bool) {
    if !*open {
        return (Action::None, false);
    }
    let screen = ctx.content_rect();
    let mut action = Action::None;
    let mut changed = false;
    let tab_id = egui::Id::unique("waters-roux-machinima-tab");
    let mut tab = ctx.data(|d| d.get_temp::<usize>(tab_id)).unwrap_or(0);
    let mut floater = Floater::new(
        WINDOW,
        "Waters Roux • Machinima Studio",
        egui::pos2(screen.center().x - 345.0, screen.bottom() - 430.0),
        Vec2::new(690.0, 390.0),
    );
    floater.min_size = Vec2::new(490.0, 310.0);
    floater.steady_size = true;
    floater.show(ctx, p, open, |ui| {
        ui.horizontal(|ui| {
            ui.strong("Creator capture & cinematic controls");
            if finalizing {
                ui.strong("SAVING MP4…");
            } else if recording {
                ui.label(egui::RichText::new(format!("REC {:02}:{:02}", elapsed.as_secs() / 60, elapsed.as_secs() % 60)).color(p.danger));
            }
        });
        widgets::tabs(
            ui,
            p,
            &mut tab,
            &[("Camera", true), ("Cinematic visuals", true), ("Record video", true)],
        );
        widgets::divider(ui, p);
        match tab {
            0 => {
                ui.label("Real camera controls • positioning, field of view, and cinematic views");
                ui.horizontal_wrapped(|ui| {
                    let button = |ui: &mut egui::Ui, icon: &str, label: &str| {
                        widgets::action_button(ui, p, icon, label, ActionKind::Flat, Vec2::new(0.0, 29.0)).clicked()
                    };
                    if button(ui, "eye", if pov { "Exit avatar POV" } else { "Avatar-eye POV" }) {
                        action = Action::TogglePov;
                    }
                    if button(ui, "camera", "Camera controls") {
                        action = Action::CameraControls;
                    }
                    if button(ui, "refresh", "Reset camera") {
                        action = Action::ResetCamera;
                    }
                    if button(ui, "camera-cog", "PhotoTools") {
                        action = Action::PhotoTools;
                    }
                    if button(ui, "camera-filled", "Snapshot") {
                        action = Action::Snapshot;
                    }
                    if button(ui, "panorama-horizontal-filled", "360 panorama") {
                        action = Action::Panorama;
                    }
                    if button(ui, "eye-off", "Hide interface") {
                        action = Action::ToggleInterface;
                    }
                });
                let mut fov = settings.camera.fov.to_degrees();
                if ui
                    .add(egui::Slider::new(&mut fov, 20.0..=100.0).text("Camera vertical field of view (degrees)"))
                    .changed()
                {
                    settings.camera.fov = fov.to_radians();
                    changed = true;
                }
                changed |= ui
                    .add(egui::Slider::new(&mut settings.camera.zoom_time, 0.0..=5.0).text("Camera transition time (s)"))
                    .changed();
                changed |= ui
                    .add(egui::Slider::new(&mut settings.camera.smoothing, 0.0..=50.0).text("Camera movement smoothing"))
                    .changed();
                changed |= widgets::switch_labeled(ui, p, &mut settings.camera.track_focus_object, "Keep focused object tracked").changed();
                widgets::divider(ui, p);
                ui.label("Cinematic skeleton follow (your own animated avatar)");
                ui.horizontal_wrapped(|ui| {
                    for (target, label) in [
                        (CinematicTarget::Off, "Off"),
                        (CinematicTarget::Head, "Head"),
                        (CinematicTarget::Torso, "Torso"),
                        (CinematicTarget::LeftArm, "Left arm"),
                        (CinematicTarget::RightArm, "Right arm"),
                    ] {
                        changed |= ui
                            .selectable_value(&mut settings.camera.cinematic_target, target, label)
                            .changed();
                    }
                });
                if settings.camera.cinematic_target != CinematicTarget::Off {
                    changed |= ui
                        .add(
                            egui::Slider::new(&mut settings.camera.cinematic_follow_half_life, 0.0..=2.0)
                                .text("Joint-follow smoothing (s)"),
                        )
                        .changed();
                    ui.small("Third-person camera only. Follows the selected rendered joint; Flycam, POV and scripted cameras keep their own behavior.");
                }
                changed |= widgets::switch_labeled(
                    ui,
                    p,
                    &mut settings.camera.mouselook_stabilize,
                    "Stabilize avatar POV (reduce head bob)",
                )
                .changed();
                ui.label("For saved camera presets and precise orbit/pan, open Camera controls.");
            }
            1 => {
                ui.label("Cinematic visual effects — settings change the actual renderer");
                egui::ScrollArea::vertical().max_height(245.0).show(ui, |ui| {
                    changed |= widgets::switch_labeled(ui, p, &mut settings.hdr_emissive, "HDR / emissive lighting").changed();
                    changed |= ui
                        .add_enabled(
                            settings.hdr_emissive,
                            egui::Slider::new(&mut settings.exposure, 0.25..=4.0).text("Exposure"),
                        )
                        .changed();
                    changed |= widgets::switch_labeled(ui, p, &mut settings.dof.enabled, "Depth of field").changed();
                    ui.add_enabled_ui(settings.dof.enabled, |ui| {
                        changed |= ui
                            .add(egui::Slider::new(&mut settings.dof.f_number, 1.0..=22.0).text("Aperture f-number"))
                            .changed();
                        changed |= ui
                            .add(egui::Slider::new(&mut settings.dof.focal_length, 10.0..=200.0).text("Lens focal length (mm)"))
                            .changed();
                    });
                    changed |= widgets::switch_labeled(ui, p, &mut settings.glow, "Object glow").changed();
                    changed |= widgets::switch_labeled(ui, p, &mut settings.bloom, "Cinematic bloom").changed();
                    if settings.bloom {
                        changed |= ui
                            .add(egui::Slider::new(&mut settings.bloom_strength, 0.05..=1.0).text("Bloom strength"))
                            .changed();
                    }
                    changed |= ui
                        .add(egui::Slider::new(&mut settings.motion_blur_strength, 0.0..=1.0).text("Cinematic camera motion blur (0 = off)"))
                        .changed();
                    ui.small("Camera panning/rotation only; recording HUDs and interface stay sharp. Off by default.");
                    changed |= widgets::switch_labeled(ui, p, &mut settings.ssr, "Screen-space reflections").changed();
                    let mut quality = settings.shadow_quality;
                    if ui.add(egui::Slider::new(&mut quality, 0..=4).text("Shadow quality")).changed() {
                        set_machinima_shadow_quality(settings, quality);
                        changed = true;
                    }
                    changed |= ui
                        .add(egui::Slider::new(&mut settings.draw_distance, 32.0..=1024.0).text("Draw distance (m)"))
                        .changed();
                    ui.collapsing("Anti-aliasing and image quality", |ui| {
                        egui::ComboBox::from_id_salt("waters-roux-machinima-antialiasing")
                            .selected_text(match settings.antialiasing {
                                0 => "Off",
                                1 => "FXAA",
                                2 => "MSAA 2×",
                                3 => "MSAA 4×",
                                4 => "MSAA 8×",
                                5 => "TAA",
                                6 => "SMAA",
                                7 => "FSR 3.1",
                                _ => "Unknown",
                            })
                            .show_ui(ui, |ui| {
                                for (mode, label) in [
                                    (0, "Off"),
                                    (1, "FXAA"),
                                    (2, "MSAA 2×"),
                                    (3, "MSAA 4×"),
                                    (4, "MSAA 8×"),
                                    (5, "TAA"),
                                    (6, "SMAA"),
                                    (7, "FSR 3.1"),
                                ] {
                                    changed |= ui.selectable_value(&mut settings.antialiasing, mode, label).changed();
                                }
                            });
                        changed |= widgets::switch_labeled(ui, p, &mut settings.specular_aa, "Specular anti-aliasing").changed();
                        if settings.antialiasing == 7 {
                            ui.label("FSR controls rendering resolution and provides its own sharpening.");
                            ui.horizontal_wrapped(|ui| {
                                for (quality, label) in [
                                    (0, "Native"),
                                    (1, "Quality"),
                                    (2, "Balanced"),
                                    (3, "Performance"),
                                    (4, "Ultra"),
                                ] {
                                    changed |= ui.selectable_value(&mut settings.fsr_quality, quality, label).changed();
                                }
                            });
                            changed |= ui
                                .add(egui::Slider::new(&mut settings.fsr_sharpness, 0..=100).text("FSR sharpness (%)"))
                                .changed();
                        }
                        changed |= ui
                            .add_enabled(
                                cas_sharpen_enabled(settings),
                                egui::Slider::new(&mut settings.sharpen, 0.0..=1.0).text("CAS sharpening"),
                            )
                            .changed();
                        if !settings.hdr_emissive {
                            ui.small("Enable HDR / Emissive to adjust tonemapping and CAS sharpening.");
                        }
                    });
                    changed |= ui
                        .add_enabled(
                            manual_render_scale_enabled(settings),
                            egui::Slider::new(&mut settings.render_scale_percent, 50..=100).text("Render resolution (%)"),
                        )
                        .changed();
                    changed |= ui
                        .add_enabled(
                            settings.hdr_emissive,
                            egui::Slider::new(&mut settings.tonemap_mix, 0.0..=1.0).text("Tonemapping strength"),
                        )
                        .changed();
                });
            }
            _ => {
                ui.strong("H.264 / MP4 video recording");
                ui.label("Captures the actual in-world renderer, not the desktop.");
                ui.horizontal_wrapped(|ui| {
                    ui.label("Recording size:");
                    for (height, label) in [
                        (0, "Full viewer"),
                        (480, "480p"),
                        (720, "720p"),
                        (1080, "1080p"),
                        (1440, "1440p"),
                        (2160, "4K"),
                    ] {
                        changed |= ui.selectable_value(&mut settings.recording.height, height, label).changed();
                    }
                });
                ui.label("Full viewer preserves the source viewport dimensions; maximize the window to capture the whole screen.");
                ui.horizontal(|ui| {
                    ui.label("Target frame rate:");
                    for fps in [24, 30, 60] {
                        changed |= ui
                            .selectable_value(&mut settings.recording.fps, fps, format!("{fps} FPS"))
                            .changed();
                    }
                });
                changed |= ui
                    .add(egui::Slider::new(&mut settings.recording.jpeg_quality, 80..=100).text("H.264 recording quality"))
                    .changed();
                changed |= widgets::switch_labeled(
                    ui,
                    p,
                    &mut settings.recording.capture_world_audio,
                    "Include in-world sounds and received voice (48 kHz stereo)",
                )
                .changed();
                changed |= widgets::switch_labeled(
                    ui,
                    p,
                    &mut settings.recording.capture_microphone,
                    "Record my own microphone / narration (48 kHz, opt-in)",
                )
                .changed();
                changed |= widgets::switch_labeled(ui, p, &mut settings.recording.capture_huds, "Include my HUD attachments").changed();
                changed |= widgets::switch_labeled(
                    ui,
                    p,
                    &mut settings.recording.capture_interface,
                    "Include viewer interface, menus and toolbars",
                )
                .changed();
                ui.label("Turn either option off for a clean cinematic capture. Changes apply when starting a new recording.");
                ui.label("Recording other residents' conversations may require their consent.");
                ui.label("World sounds and your own microphone can be enabled independently. With both selected they are mixed into one AAC track.");
                ui.label("Uses the selected microphone in Preferences > Sound and Voice. If it is unavailable, recording will not start.");
                ui.label("Records H.264 video to a real MP4 file; optional in-world audio is encoded as AAC. Recording requires the bundled FFmpeg encoder.");
                ui.label("MP4 finishing happens in the background after Stop. Wait for the Saved MP4 message before exiting.");
                ui.label("Quick Record: press F9 to start or stop anywhere in-world, including Flycam. Change the key in Preferences > Shortcuts.");
                ui.label("Your microphone is recorded locally only when its checkbox is enabled; other desktop applications are not captured.");
                ui.label("If other residents' voices can be heard, obtain their consent before recording or sharing.");
                ui.horizontal(|ui| {
                    if finalizing {
                        ui.strong("Finalizing H.264 video and AAC audio… keep the viewer open.");
                    } else if recording {
                        if widgets::action_button(
                            ui,
                            p,
                            "player-stop-filled",
                            "Stop and save video",
                            ActionKind::Primary,
                            Vec2::new(0.0, 32.0),
                        )
                        .clicked()
                        {
                            action = Action::StopRecording;
                        }
                    } else if widgets::action_button(ui, p, "video", "Start recording", ActionKind::Primary, Vec2::new(0.0, 32.0)).clicked()
                    {
                        action = Action::StartRecording;
                    }
                });
            }
        }
    });
    ctx.data_mut(|d| d.insert_temp(tab_id, tab));
    if changed {
        settings.recording.sanitize();
        settings.dof.sanitize();
    }
    (action, changed)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn machinima_shadow_slider_updates_the_actual_renderer_switch() {
        let mut settings = Settings::default();
        set_machinima_shadow_quality(&mut settings, 0);
        assert_eq!(settings.shadow_quality, 0);
        assert!(!settings.shadows);
        set_machinima_shadow_quality(&mut settings, 3);
        assert_eq!(settings.shadow_quality, 3);
        assert!(settings.shadows);
    }

    #[test]
    fn machinima_fsr_prevents_conflicting_scale_and_cas_controls() {
        let mut settings = Settings {
            antialiasing: 7,
            hdr_emissive: true,
            render_scale_percent: 82,
            sharpen: 0.6,
            ..Settings::default()
        };
        assert!(!manual_render_scale_enabled(&settings));
        assert!(!cas_sharpen_enabled(&settings));
        assert_eq!(settings.render_scale_percent, 82);
        assert_eq!(settings.sharpen, 0.6);
        settings.antialiasing = 6;
        assert!(manual_render_scale_enabled(&settings));
        assert!(cas_sharpen_enabled(&settings));
        settings.hdr_emissive = false;
        assert!(!cas_sharpen_enabled(&settings));
        assert!(manual_render_scale_enabled(&settings));
    }

    #[test]
    fn machinima_image_quality_settings_survive_restart() {
        let settings = Settings {
            antialiasing: 7,
            fsr_quality: 2,
            fsr_sharpness: 63,
            specular_aa: true,
            ..Settings::default()
        };
        let loaded: Settings = serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert_eq!(loaded.antialiasing, 7);
        assert_eq!(loaded.fsr_quality, 2);
        assert_eq!(loaded.fsr_sharpness, 63);
        assert!(loaded.specular_aa);
    }

    #[test]
    fn machinima_actions_are_explicit() {
        assert_eq!(Action::default(), Action::None);
        assert_ne!(Action::TogglePov, Action::Snapshot);
        assert_ne!(Action::PhotoTools, Action::CameraControls);
    }
}
