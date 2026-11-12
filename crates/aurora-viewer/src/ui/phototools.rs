//! Dedicated photography controls. Snapshot, general camera Controls, and
//! PhotoTools are separate tools; the latter edits the *real* renderer
//! settings rather than forwarding its toolbar button to Snapshot.

use super::{
    Panels,
    widgets::{self, ActionKind, Floater},
};
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
    let mut open_machinima = false;
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
            changed |= ui
                .add_enabled(
                    settings.hdr_emissive,
                    egui::Slider::new(&mut settings.exposure, 0.25..=4.0).text("Exposure"),
                )
                .changed();
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
                if widgets::action_button(ui, p, "video", "Machinima bar", ActionKind::Flat, Vec2::new(0.0, 29.0)).clicked() {
                    open_machinima = true;
                }
            });
            ui.label("EEP and personal lighting open their full native editors.");
            widgets::divider(ui, p);
            ui.label("In-world editing feedback");
            changed |= widgets::switch_labeled(ui, p, &mut settings.build.show_selection_beam, "Hand-to-object selection beam").changed();
            if settings.build.show_selection_beam {
                changed |=
                    widgets::switch_labeled(ui, p, &mut settings.build.selection_beam_animated, "Animate selection particles").changed();
                ui.collapsing("Selection Beam Studio", |ui| {
                    ui.label("Choose a local particle style for the hand-to-object editing beam.");
                    ui.horizontal_wrapped(|ui| {
                        for (style, label) in [
                            ("dots", "Dots"),
                            ("sparkles", "Sparkles"),
                            ("stars", "Stars"),
                            ("hearts", "Hearts"),
                            ("custom", "My PNG"),
                        ] {
                            changed |= ui
                                .selectable_value(&mut settings.build.selection_beam_style, style.to_owned(), label)
                                .changed();
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Particle color");
                        changed |= widgets::color_button(
                            ui,
                            p,
                            "selection-beam-custom-color",
                            &mut settings.build.selection_beam_color,
                            true,
                        )
                        .changed();
                    });
                    changed |= ui
                        .add(egui::Slider::new(&mut settings.build.selection_beam_size, 2.0..=20.0).text("Particle size"))
                        .changed();
                    changed |= ui
                        .add(egui::Slider::new(&mut settings.build.selection_beam_density, 25..=200).text("Density (%)"))
                        .changed();
                    changed |= ui
                        .add(egui::Slider::new(&mut settings.build.selection_beam_speed, 0.1..=4.0).text("Animation speed"))
                        .changed();
                    if settings.build.selection_beam_style == "custom" {
                        ui.horizontal(|ui| {
                            if ui.button("Import particle PNG...").clicked()
                                && let Some(file) = rfd::FileDialog::new()
                                    .add_filter("Transparent PNG", &["png"])
                                    .pick_file()
                            {
                                settings.build.selection_beam_texture = file.to_string_lossy().into_owned();
                                changed = true;
                            }
                            if ui.button("Clear image").clicked() {
                                settings.build.selection_beam_texture.clear();
                                changed = true;
                            }
                        });
                        if let Some(texture) = crate::build::selection_beam::custom_texture(
                            ctx,
                            &settings.build.selection_beam_texture,
                        ) {
                            ui.add(egui::Image::new((texture.id(), egui::Vec2::splat(52.0))));
                        } else if !settings.build.selection_beam_texture.is_empty() {
                            ui.label("PNG could not be loaded. Use an image under 8 MB and 2048 × 2048 pixels.");
                        }
                        ui.label("Local PNG images only. Firestorm XML preset import is not supported yet.");
                    }
                    if ui.button("Restore default gold beam").clicked() {
                        settings.build.selection_beam_style = "dots".into();
                        settings.build.selection_beam_color = [253, 181, 21, 210];
                        settings.build.selection_beam_size = 4.0;
                        settings.build.selection_beam_density = 100;
                        settings.build.selection_beam_speed = 1.0;
                        changed = true;
                    }
                });
            }
        } else {
            ui.label("Photography-specific lens and depth-of-field controls");
            changed |= widgets::switch_labeled(ui, p, &mut settings.dof.enabled, "Enable depth of field").changed();
            ui.add_enabled_ui(settings.dof.enabled, |ui| {
                changed |= ui
                    .add(egui::Slider::new(&mut settings.dof.f_number, 1.0..=22.0).text("Aperture (f-number)"))
                    .changed();
                changed |= ui
                    .add(egui::Slider::new(&mut settings.dof.focal_length, 10.0..=200.0).text("Focal length (mm)"))
                    .changed();
                changed |= ui
                    .add(egui::Slider::new(&mut settings.dof.transition, 0.1..=5.0).text("Focus transition (s)"))
                    .changed();
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
    if open_machinima {
        panels.machinima = true;
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
