//! Dedicated creator cinematography panel. This is NOT general Camera Controls:
//! it exposes live camera/lighting render settings and an opt-in native recorder.

use super::widgets::{self, ActionKind, Floater};
use crate::{settings::Settings, theme::Palette};
use egui::{Context, Vec2};

pub const WINDOW: &str = "waters_roux_machinima";

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
            if recording {
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
                ui.label("For saved camera presets and precise orbit/pan, open Camera controls.");
            }
            1 => {
                ui.label("Cinematic visual effects — settings change the actual renderer");
                egui::ScrollArea::vertical().max_height(245.0).show(ui, |ui| {
                    changed |= widgets::switch_labeled(ui, p, &mut settings.hdr_emissive, "HDR / emissive lighting").changed();
                    changed |= ui
                        .add(egui::Slider::new(&mut settings.exposure, 0.25..=4.0).text("Exposure"))
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
                    changed |= widgets::switch_labeled(ui, p, &mut settings.ssr, "Screen-space reflections").changed();
                    changed |= ui
                        .add(egui::Slider::new(&mut settings.shadow_quality, 0..=4).text("Shadow quality"))
                        .changed();
                    changed |= ui
                        .add(egui::Slider::new(&mut settings.draw_distance, 32.0..=512.0).text("Draw distance (m)"))
                        .changed();
                    changed |= ui
                        .add(egui::Slider::new(&mut settings.render_scale_percent, 50..=100).text("Render resolution (%)"))
                        .changed();
                    changed |= ui
                        .add(egui::Slider::new(&mut settings.tonemap_mix, 0.0..=1.0).text("Tonemapping strength"))
                        .changed();
                });
            }
            _ => {
                ui.strong("High-quality recording configuration");
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
                    .add(egui::Slider::new(&mut settings.recording.jpeg_quality, 80..=100).text("Frame image quality"))
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
                    &mut settings.recording.capture_huds,
                    "Include my HUD attachments",
                )
                .changed();
                changed |= widgets::switch_labeled(
                    ui,
                    p,
                    &mut settings.recording.capture_interface,
                    "Include viewer interface, menus and toolbars",
                )
                .changed();
                ui.label("Turn either option off for a clean cinematic capture. Changes apply when starting a new recording.");
                ui.label("Recording other residents' conversations may require their consent.");
                ui.label("Your microphone narration and unrelated desktop audio are not captured yet.");
                ui.label("AVI/MJPEG + PCM is experimental and can produce very large files. H.264/MP4 recording is not implemented yet.");
                ui.horizontal(|ui| {
                    if recording {
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
    fn machinima_actions_are_explicit() {
        assert_eq!(Action::default(), Action::None);
        assert_ne!(Action::TogglePov, Action::Snapshot);
        assert_ne!(Action::PhotoTools, Action::CameraControls);
    }
}
