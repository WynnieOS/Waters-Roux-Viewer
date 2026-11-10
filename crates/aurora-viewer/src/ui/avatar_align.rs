//! Compass controls for the local avatar, separate from camera controls.
//! Cardinal headings use +X as east and +Y as north, matching AgentState::yaw.

use super::widgets::Floater;
use crate::theme::Palette;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    RotateTo(f32),
    FaceNearest,
}

pub fn show(ctx: &egui::Context, p: &Palette, open: &mut bool, yaw: f32) -> Option<Action> {
    let mut action = None;
    Floater::new("avatar_align", "Avatar Alignment", egui::pos2(540.0, 150.0), egui::vec2(345.0, 245.0))
        .show(ctx, p, open, |ui| {
            ui.label("Turn your avatar without moving or changing the camera.");
            ui.add_space(8.0);
            for row in [
                [("NW", 135.0_f32), ("N", 90.0), ("NE", 45.0)],
                [("W", 180.0), ("E", 0.0), ("SE", -45.0)],
            ] {
                ui.horizontal(|ui| {
                    for (name, angle) in row {
                        if ui.add_sized([74.0, 30.0], egui::Button::new(name)).clicked() {
                            action = Some(Action::RotateTo(angle.to_radians()));
                        }
                    }
                });
            }
            ui.horizontal(|ui| {
                if ui.add_sized([74.0, 30.0], egui::Button::new("SW")).clicked() {
                    action = Some(Action::RotateTo((-135.0_f32).to_radians()));
                }
                if ui.add_sized([74.0, 30.0], egui::Button::new("S")).clicked() {
                    action = Some(Action::RotateTo((-90.0_f32).to_radians()));
                }
                if ui.add_sized([74.0, 30.0], egui::Button::new("Face nearest")).clicked() {
                    action = Some(Action::FaceNearest);
                }
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Turn 15° left").clicked() {
                    action = Some(Action::RotateTo(yaw + 15.0_f32.to_radians()));
                }
                if ui.button("Turn 15° right").clicked() {
                    action = Some(Action::RotateTo(yaw - 15.0_f32.to_radians()));
                }
            });
        });
    action
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compass_angles_use_correct_world_axes() {
        let east = 0.0_f32;
        let north = 90.0_f32.to_radians();
        let west = 180.0_f32.to_radians();
        assert!((east.cos() - 1.0).abs() < 1e-6);
        assert!((north.sin() - 1.0).abs() < 1e-6);
        assert!((west.cos() + 1.0).abs() < 1e-6);
    }
}
