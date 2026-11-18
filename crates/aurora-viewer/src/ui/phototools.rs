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

// Match the V3 renderer and saved graphics limit. PhotoTools must never
// truncate a resident's 512m+ draw distance when another control changes.
const PHOTO_DRAW_DISTANCE_MIN: f32 = 32.0;
const PHOTO_DRAW_DISTANCE_MAX: f32 = 1024.0;

fn clamp_photo_draw_distance(distance: f32) -> f32 {
    distance.clamp(PHOTO_DRAW_DISTANCE_MIN, PHOTO_DRAW_DISTANCE_MAX)
}

/// The renderer keeps a separate shadow-enable flag; changing quality from
/// PhotoTools must update both exactly as Preferences > Graphics does.
fn set_photo_shadow_quality(settings: &mut Settings, quality: u8) {
    settings.shadow_quality = quality.min(4);
    settings.shadows = settings.shadow_quality > 0;
}

/// FSR's own upscaler controls render resolution and uses its own RCAS
/// sharpening. Never show independently editable controls that conflict
/// with the actual active image-quality pipeline.
fn photo_manual_render_scale_enabled(settings: &Settings) -> bool {
    settings.antialiasing != 7
}

fn photo_cas_sharpen_enabled(settings: &Settings) -> bool {
    settings.hdr_emissive && settings.antialiasing != 7
}

/// Save to the existing persisted graphics/photo library, without silently
/// replacing a resident's named profile or storing account/device settings.
fn create_photo_preset(settings: &mut Settings, name: &str) -> bool {
    use crate::graphics_presets::saved::{self, Saved};

    let name = name.trim();
    if !saved::valid_name(name) || settings.graphics_saved.contains_key(name) {
        return false;
    }
    let snapshot = Saved::capture(settings);
    settings.graphics_saved.insert(name.to_owned(), snapshot);
    settings.graphics_saved_active = Some(name.to_owned());
    true
}

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
    let preset_name_id = egui::Id::unique("waters-roux-photo-preset-name");
    let mut preset_name = ctx.data(|d| d.get_temp::<String>(preset_name_id)).unwrap_or_default();
    let screen = ctx.content_rect();

    Floater::new(
        WINDOW,
        "Waters Roux PhotoTools",
        egui::pos2(screen.right() - 440.0, screen.top() + 90.0),
        Vec2::new(400.0, 460.0),
    )
    // PhotoTools has expandable sections and two tabs. Auto-sizing its height
    // on every frame causes repeated layout changes as the resident works.
    // Keep an independently resizable, persistent floater instead.
    .steady_size()
    .show(ctx, p, &mut open, |ui| {
        widgets::tabs(ui, p, &mut tab, &[("Photo & Light", true), ("Photo Camera", true)]);
        widgets::divider(ui, p);
        // The content can be much taller than the floater when controls
        // expand; scroll within its stable viewport rather than moving or
        // growing the entire window each frame.
        egui::ScrollArea::vertical()
            .id_salt("waters-roux-phototools-content-scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
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
                        .add(
                            egui::Slider::new(&mut settings.draw_distance, PHOTO_DRAW_DISTANCE_MIN..=PHOTO_DRAW_DISTANCE_MAX)
                                .text("Draw distance (m)"),
                        )
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
                    ui.collapsing("Advanced lighting, reflections and finishing", |ui| {
                        ui.label("Live renderer settings; demanding effects can reduce FPS.");
                        changed |= widgets::switch_labeled(ui, p, &mut settings.glow, "Object glow").changed();
                        changed |= widgets::switch_labeled(ui, p, &mut settings.bloom, "Bloom").changed();
                        changed |= ui
                            .add_enabled(
                                settings.bloom,
                                egui::Slider::new(&mut settings.bloom_strength, 0.05..=1.0)
                                    .text("Bloom strength"),
                            )
                            .changed();
                        let mut shadow_quality = settings.shadow_quality;
                        if ui
                            .add(egui::Slider::new(&mut shadow_quality, 0..=4).text("Shadow quality"))
                            .changed()
                        {
                            set_photo_shadow_quality(settings, shadow_quality);
                            changed = true;
                        }
                        ui.add_enabled_ui(settings.shadow_quality > 0, |ui| {
                            changed |= widgets::switch_labeled(
                                ui,
                                p,
                                &mut settings.contact_shadows,
                                "Contact shadows",
                            )
                            .changed();
                            changed |= widgets::switch_labeled(
                                ui,
                                p,
                                &mut settings.projector_shadows,
                                "Projector shadows",
                            )
                            .changed();
                        });
                        changed |= ui
                            .add(egui::Slider::new(&mut settings.ssao, 0..=2).text("Ambient occlusion"))
                            .changed();
                        changed |= widgets::switch_labeled(
                            ui,
                            p,
                            &mut settings.ssr,
                            "Screen-space reflections",
                        )
                        .changed();
                        ui.add_enabled_ui(settings.hdr_emissive, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.label("PBR tonemapping:");
                                changed |= ui
                                    .selectable_value(&mut settings.tonemapper, 0, "Neutral")
                                    .changed();
                                changed |= ui
                                    .selectable_value(&mut settings.tonemapper, 1, "ACES")
                                    .changed();
                            });
                            changed |= ui
                                .add(egui::Slider::new(&mut settings.tonemap_mix, 0.0..=1.0).text("Tonemapping blend"))
                                .changed();
                        });
                        ui.collapsing("Cinematic image quality / anti-aliasing", |ui| {
                            ui.label("Choose the live renderer's native anti-aliasing pipeline.");
                            egui::ComboBox::from_id_salt("waters-roux-photo-antialiasing")
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
                                    for (mode, name) in [
                                        (0, "Off"),
                                        (1, "FXAA"),
                                        (2, "MSAA 2×"),
                                        (3, "MSAA 4×"),
                                        (4, "MSAA 8×"),
                                        (5, "TAA"),
                                        (6, "SMAA"),
                                        (7, "FSR 3.1"),
                                    ] {
                                        changed |= ui.selectable_value(&mut settings.antialiasing, mode, name).changed();
                                    }
                                });
                            changed |= widgets::switch_labeled(ui, p, &mut settings.specular_aa, "Specular anti-aliasing").changed();
                            if settings.antialiasing == 7 {
                                ui.label("FSR controls the world render resolution and its own sharpening.");
                                ui.horizontal_wrapped(|ui| {
                                    for (mode, label) in [
                                        (0, "Native"),
                                        (1, "Quality"),
                                        (2, "Balanced"),
                                        (3, "Performance"),
                                        (4, "Ultra"),
                                    ] {
                                        changed |= ui.selectable_value(&mut settings.fsr_quality, mode, label).changed();
                                    }
                                });
                                changed |= ui
                                    .add(egui::Slider::new(&mut settings.fsr_sharpness, 0..=100).text("FSR sharpness (%)"))
                                    .changed();
                            }
                            changed |= ui
                                .add_enabled(
                                    photo_cas_sharpen_enabled(settings),
                                    egui::Slider::new(&mut settings.sharpen, 0.0..=1.0).text("CAS sharpening"),
                                )
                                .changed();
                            if !settings.hdr_emissive {
                                ui.small("Enable HDR / Emissive to adjust tonemapping and CAS sharpening.");
                            }
                        });
                        changed |= ui
                            .add_enabled(
                                photo_manual_render_scale_enabled(settings),
                                egui::Slider::new(&mut settings.render_scale_percent, 50..=100).text("Render resolution (%)"),
                            )
                            .changed();
                    });
                    ui.collapsing("Saved photo and graphics presets", |ui| {
                        ui.label("Profiles are shared with Preferences > Graphics, saved across restarts.");
                        let selected = crate::graphics_presets::saved::selected(settings)
                            .unwrap_or("Choose a saved profile")
                            .to_owned();
                        let names: Vec<String> = settings.graphics_saved.keys().cloned().collect();
                        egui::ComboBox::from_id_salt("waters-roux-photo-profiles")
                            .selected_text(selected)
                            .show_ui(ui, |ui| {
                                for name in names {
                                    let active = crate::graphics_presets::saved::selected(settings)
                                        == Some(name.as_str());
                                    if ui.selectable_label(active, &name).clicked() {
                                        changed |= crate::graphics_presets::saved::select(settings, &name);
                                    }
                                }
                            });
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut preset_name)
                                    .hint_text("New profile name"),
                            );
                            let valid = crate::graphics_presets::saved::valid_name(&preset_name)
                                && !settings.graphics_saved.contains_key(preset_name.trim());
                            if ui
                                .add_enabled(valid, egui::Button::new("Save new"))
                                .clicked()
                            {
                                changed |= create_photo_preset(settings, &preset_name);
                            }
                        });
                        let active = crate::graphics_presets::saved::selected(settings).is_some();
                        if ui
                            .add_enabled(active, egui::Button::new("Update selected profile"))
                            .clicked()
                        {
                            changed |= crate::graphics_presets::saved::save_selected(settings);
                        }
                        ui.small("Selecting a profile restores its saved rendering and lens settings. Existing profiles are never overwritten by Save new.");
                    });
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
                                changed |=
                                    widgets::color_button(ui, p, "selection-beam-custom-color", &mut settings.build.selection_beam_color, true)
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
                                        && let Some(file) = rfd::FileDialog::new().add_filter("Transparent PNG", &["png"]).pick_file()
                                    {
                                        settings.build.selection_beam_texture = file.to_string_lossy().into_owned();
                                        changed = true;
                                    }
                                    if ui.button("Clear image").clicked() {
                                        settings.build.selection_beam_texture.clear();
                                        changed = true;
                                    }
                                });
                                if let Some(texture) = crate::build::selection_beam::custom_texture(ctx, &settings.build.selection_beam_texture) {
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
    });
    ctx.data_mut(|d| d.insert_temp(preset_name_id, preset_name));
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
        settings.draw_distance = clamp_photo_draw_distance(settings.draw_distance);
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phototools_preserves_v3_long_range_draw_distance() {
        assert_eq!(clamp_photo_draw_distance(768.0), 768.0);
        assert_eq!(clamp_photo_draw_distance(1024.0), 1024.0);
        assert_eq!(clamp_photo_draw_distance(2048.0), 1024.0);
        assert_eq!(clamp_photo_draw_distance(16.0), 32.0);
    }

    #[test]
    fn phototools_shadow_quality_controls_the_actual_renderer_flag() {
        let mut settings = Settings::default();
        set_photo_shadow_quality(&mut settings, 0);
        assert_eq!(settings.shadow_quality, 0);
        assert!(!settings.shadows);
        set_photo_shadow_quality(&mut settings, 3);
        assert_eq!(settings.shadow_quality, 3);
        assert!(settings.shadows);
        set_photo_shadow_quality(&mut settings, 9);
        assert_eq!(settings.shadow_quality, 4);
        assert!(settings.shadows);
    }

    #[test]
    fn photo_presets_persist_and_restore_lighting_without_overwriting() {
        let mut settings = Settings {
            draw_distance: 768.0,
            ..Settings::default()
        };
        set_photo_shadow_quality(&mut settings, 3);
        settings.bloom = true;
        settings.dof.enabled = true;
        assert!(create_photo_preset(&mut settings, " Bayou Portrait "));
        assert!(!create_photo_preset(&mut settings, "Bayou Portrait"));
        assert!(!create_photo_preset(&mut settings, "  "));
        assert_eq!(settings.graphics_saved.len(), 1);
        settings.draw_distance = 64.0;
        set_photo_shadow_quality(&mut settings, 0);
        settings.bloom = false;
        settings.dof.enabled = false;
        let json = serde_json::to_string(&settings).expect("save photo profiles");
        let mut restored: Settings = serde_json::from_str(&json).expect("load photo profiles");
        assert!(crate::graphics_presets::saved::select(&mut restored, "Bayou Portrait",));
        assert_eq!(restored.draw_distance, 768.0);
        assert_eq!(restored.shadow_quality, 3);
        assert!(restored.shadows);
        assert!(restored.bloom);
        assert!(restored.dof.enabled);
    }

    #[test]
    fn phototools_fsr_disables_conflicting_render_scale_and_cas_sharpening() {
        let mut settings = Settings {
            antialiasing: 7,
            hdr_emissive: true,
            render_scale_percent: 77,
            sharpen: 0.6,
            ..Settings::default()
        };
        assert!(!photo_manual_render_scale_enabled(&settings));
        assert!(!photo_cas_sharpen_enabled(&settings));
        assert_eq!(settings.render_scale_percent, 77);
        assert_eq!(settings.sharpen, 0.6);

        settings.antialiasing = 5;
        assert!(photo_manual_render_scale_enabled(&settings));
        assert!(photo_cas_sharpen_enabled(&settings));
        settings.hdr_emissive = false;
        assert!(!photo_cas_sharpen_enabled(&settings));
        assert!(photo_manual_render_scale_enabled(&settings));
    }

    #[test]
    fn phototools_quality_modes_persist_without_resetting_existing_profiles() {
        let mut settings = Settings {
            antialiasing: 7,
            fsr_quality: 2,
            fsr_sharpness: 63,
            specular_aa: true,
            ..Settings::default()
        };
        assert!(create_photo_preset(&mut settings, "Creator Film"));
        settings.antialiasing = 1;
        settings.fsr_quality = 0;
        settings.fsr_sharpness = 0;
        let stored: Settings = serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert!(stored.graphics_saved.contains_key("Creator Film"));
        assert_eq!(stored.antialiasing, 1);
        assert_eq!(stored.fsr_sharpness, 0);
    }

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
