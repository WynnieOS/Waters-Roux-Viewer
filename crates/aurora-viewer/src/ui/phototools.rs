//! Dedicated photography controls. Snapshot, general camera Controls, and
//! PhotoTools are separate tools; the latter edits the *real* renderer
//! settings rather than forwarding its toolbar button to Snapshot.

use super::{Panels, widgets::{self, ActionKind, Floater}};
use crate::{dof::FocusMode, settings::Settings, theme::Palette};
use egui::{Context, Vec2};

pub const WINDOW: &str = "waters_roux_phototools";

/// Returns whether settings changed and require persistence.
/// The toolbar owns visibility and current tab, so no phantom window opens.
pub fn show(ctx: &Context, p: &Palette, panels: &mut Panels, settings: &mut Settings) -> bool {
    if !panels.photo_tools {
        return false;
    }
    let mut open = panels.photo_tools;
    let mut tab = usize::from(panels.photo_tools_camera_tab);
    let mut changed = false;
    let mut open_snapshot = false;
    let mut open_environment = false;
    let mut open_lighting = false;
    let mut open_camera = false;
    let screen = ctx.content_rect();

    Floater::new(
        WINDOW,
        "Waters Roux PhotoTools",
        egui::pos2(screen.right() - 440.0, screen.top() + 90.0),
        Vec2::new(400.0, 460.0),
    )
    .auto_height()
    .show(ctx, p, &mut open, |ui| {
        widgets::tabs(ui, p, &mut tab, &[("Photo & Light", true), ("Photo Camera", true)]);
        widgets::divider(ui, p);
        if tab == 0 {
            ui.label("Live world lighting and render adjustments");
            changed |= widgets::switch_labeled(ui, p, &mut settings.hdr_emissive, "HDR / Emissive").changed();
            changed |= ui.add_enabled(
                settings.hdr_emissive,
                egui::Slider::new(&mut settings.exposure, 0.25..=4.0).text("Exposure"),
            ).changed();
            changed |= ui
                .add(egui::Slider::new(&mut settings.draw_distance, 32.0..=512.0).text("Draw distance (m)"))
                .changed();
            ui.horizontal_wrapped(|ui| {
                if widgets::action_button(ui, p, "sun", "EEP / WindLight", ActionKind::Flat, Vec2::new(0.0, 29.0)).clicked() {
                    open_environment = true;
                }
                if widgets::action_button(ui, p, "bulb", "Personal lighting", ActionKind::Flat, Vec2::new(0.0, 29.0)).clicked() {
                    open_lighting = true;
                }
            });
            ui.label("EEP and personal lighting open their full native editors.");
        } else {
            ui.label("Photography-specific lens and depth-of-field controls");
            changed |= widgets::switch_labeled(ui, p, &mut settings.dof.enabled, "Enable depth of field").changed();
            ui.add_enabled_ui(settings.dof.enabled, |ui| {
                changed |= ui.add(egui::Slider::new(&mut settings.dof.f_number, 1.0..=22.0).text("Aperture (f-number)")).changed();
                changed |= ui
                    .add(egui::Slider::new(&mut settings.dof.focal_length, 10.0..=200.0).text("Focal length (mm)"))
                    .changed();
                changed |= ui.add(egui::Slider::new(&mut settings.dof.transition, 0.1..=5.0).text("Focus transition (s)")).changed();
                ui.horizontal_wrapped(|ui| {
                    ui.label("Focus target");
                    for (mode, label) in [
                        (FocusMode::Camera, "Camera"),
                        (FocusMode::Pointer, "Pointer"),
                        (FocusMode::Click, "Click"),
                        (FocusMode::Locked, "Locked"),
                    ] {
                        changed |= ui.selectable_value(&mut settings.dof.focus, mode, label).changed();
                    }
                });
            });
            ui.horizontal_wrapped(|ui| {
                if widgets::action_button(ui, p, "video", "Camera movement", ActionKind::Flat, Vec2::new(0.0, 29.0)).clicked() {
                    open_camera = true;
                }
                if widgets::action_button(ui, p, "camera", "Snapshot", ActionKind::Flat, Vec2::new(0.0, 29.0)).clicked() {
                    open_snapshot = true;
                }
            });
        }
    });
    panels.photo_tools = open;
    panels.photo_tools_camera_tab = tab == 1;
    if open_snapshot {
        panels.snapshot = true;
    }
    if open_environment {
        panels.environment = true;
    }
    if open_lighting {
        panels.personal_lighting = true;
    }
    if open_camera {
        panels.camera_controls = true;
    }
    if changed {
        settings.dof.sanitize();
        settings.exposure = settings.exposure.clamp(0.25, 4.0);
        settings.draw_distance = settings.draw_distance.clamp(32.0, 512.0);
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phototools_has_its_own_panel_state() {
        let mut panels = Panels::from_settings(&Settings::default());
        assert!(!panels.photo_tools);
        panels.photo_tools = true;
        panels.photo_tools_camera_tab = true;
        assert!(!panels.snapshot);
        assert!(!panels.camera_controls);
    }
}
