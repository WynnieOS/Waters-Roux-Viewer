//! Dedicated, movable cinematography control bar. Every enabled button
//! invokes a real camera, photo or user-initiated video recorder action.

use super::widgets::{self, ActionKind, Floater};
use crate::theme::Palette;
use egui::{Context, Vec2};

pub const WINDOW: &str = "waters_roux_machinima";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Action {
    #[default]
    None,
    /// Avatar-eye POV uses the actual resident camera's first-person mode.
    TogglePov,
    ResetCamera,
    CameraControls,
    PhotoToolsCamera,
    Snapshot,
    Panorama,
    ToggleInterface,
    StartRecording,
    StopRecording,
}

pub fn show(ctx: &Context, p: &Palette, open: &mut bool, pov: bool, recording: bool, elapsed: std::time::Duration) -> Action {
    if !*open {
        return Action::None;
    }
    let screen = ctx.content_rect();
    let mut action = Action::None;
    let mut floater = Floater::new(
        WINDOW,
        "Waters Roux Machinima Bar",
        egui::pos2(screen.center().x - 345.0, screen.bottom() - 160.0),
        Vec2::new(690.0, 128.0),
    );
    floater.min_size = Vec2::new(470.0, 112.0);
    floater.steady_size = true;
    floater.show(ctx, p, open, |ui| {
        ui.horizontal_wrapped(|ui| {
            if recording {
                ui.strong(format!("REC • {:02}:{:02}", elapsed.as_secs() / 60, elapsed.as_secs() % 60));
                if widgets::action_button(ui, p, "player-stop-filled", "Stop and save AVI", ActionKind::Primary, Vec2::new(0.0, 29.0)).clicked() {
                    action = Action::StopRecording;
                }
            } else if widgets::action_button(ui, p, "video", "Record silent video", ActionKind::Primary, Vec2::new(0.0, 29.0)).clicked() {
                action = Action::StartRecording;
            }
            let btn = |ui: &mut egui::Ui, icon: &str, label: &str| {
                widgets::action_button(ui, p, icon, label, ActionKind::Flat, Vec2::new(0.0, 29.0)).clicked()
            };
            if btn(ui, "eye", if pov { "Exit avatar POV" } else { "Avatar-eye POV" }) {
                action = Action::TogglePov;
            }
            if btn(ui, "video", "Camera controls") {
                action = Action::CameraControls;
            }
            if btn(ui, "refresh", "Reset camera") {
                action = Action::ResetCamera;
            }
            if btn(ui, "camera-cog", "Photo camera") {
                action = Action::PhotoToolsCamera;
            }
            if btn(ui, "camera", "Snapshot") {
                action = Action::Snapshot;
            }
            if btn(ui, "panorama-horizontal-filled", "360 capture") {
                action = Action::Panorama;
            }
            if btn(ui, "eye-off", "Hide interface") {
                action = Action::ToggleInterface;
            }
        });
        ui.label("Video saves locally as silent MJPEG AVI (up to 720p/12 fps). Voice and desktop audio are not recorded.");
    });
    action
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn machinima_actions_are_explicit() {
        assert_eq!(Action::default(), Action::None);
        assert_ne!(Action::TogglePov, Action::Snapshot);
    }
}
