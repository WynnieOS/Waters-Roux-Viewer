//! Waters Roux's saved Everyday / Studio / Creator workspaces.
//!
//! Aurora already serializes window positions and sizes in ui_layout.ron.
//! We take separate snapshots, plus each mode's relevant open windows, so
//! moving panels around in Studio does not destroy the Everyday layout.
use super::{Panels, layout};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    #[default]
    Everyday,
    Studio,
    Creator,
}

impl Mode {
    #[cfg(test)]
    pub const ALL: [Mode; 3] = [Mode::Everyday, Mode::Studio, Mode::Creator];

    pub fn slug(self) -> &'static str {
        match self {
            Mode::Everyday => "everyday",
            Mode::Studio => "studio",
            Mode::Creator => "creator",
        }
    }
}

/// Only the purpose-specific panels are changed by a workspace switch.
/// Conversations, open transactions and unrelated dialogs stay protected.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct FocusPanels {
    chat: bool,
    inventory: bool,
    minimap: bool,
    people: bool,
    snapshot: bool,
    camera_controls: bool,
    animation_explorer: bool,
    environment: bool,
    ao: bool,
    about_land: bool,
    region_estate: bool,
    area_search: bool,
    quick_prefs: bool,
}

impl Default for FocusPanels {
    fn default() -> Self {
        Self::defaults(Mode::Everyday)
    }
}

impl FocusPanels {
    fn defaults(mode: Mode) -> Self {
        Self {
            chat: matches!(mode, Mode::Everyday),
            inventory: true,
            minimap: matches!(mode, Mode::Everyday),
            people: matches!(mode, Mode::Everyday),
            snapshot: matches!(mode, Mode::Studio),
            camera_controls: matches!(mode, Mode::Studio),
            animation_explorer: matches!(mode, Mode::Studio),
            environment: matches!(mode, Mode::Studio),
            ao: matches!(mode, Mode::Studio),
            about_land: matches!(mode, Mode::Creator),
            region_estate: false,
            area_search: matches!(mode, Mode::Creator),
            quick_prefs: matches!(mode, Mode::Creator),
        }
    }

    fn capture(p: &Panels) -> Self {
        Self {
            chat: p.chat,
            inventory: p.inventory,
            minimap: p.minimap,
            people: p.people,
            snapshot: p.snapshot,
            camera_controls: p.camera_controls,
            animation_explorer: p.animation_explorer,
            environment: p.environment,
            ao: p.ao,
            about_land: p.about_land,
            region_estate: p.region_estate,
            area_search: p.area_search,
            quick_prefs: p.quick_prefs,
        }
    }

    fn apply(&self, p: &mut Panels) {
        p.chat = self.chat;
        p.inventory = self.inventory;
        p.minimap = self.minimap;
        p.people = self.people;
        p.snapshot = self.snapshot;
        p.camera_controls = self.camera_controls;
        p.animation_explorer = self.animation_explorer;
        p.environment = self.environment;
        p.ao = self.ao;
        p.about_land = self.about_land;
        p.region_estate = self.region_estate;
        p.area_search = self.area_search;
        p.quick_prefs = self.quick_prefs;
    }
}

fn directory() -> PathBuf {
    crate::settings::config_dir().join("workspaces")
}

fn positions(mode: Mode) -> PathBuf {
    directory().join(format!("{}.ron", mode.slug()))
}

fn windows(mode: Mode) -> PathBuf {
    directory().join(format!("{}.json", mode.slug()))
}

/// Capture the current arrangement and the relevant open panels for one
/// workspace. The existing atomic writer protects both files from partial
/// writes on shutdown and Windows indexing races.
pub fn capture(ctx: &egui::Context, mode: Mode, panels: &Panels) -> Result<(), String> {
    std::fs::create_dir_all(directory()).map_err(|e| e.to_string())?;
    let text = layout::text(ctx).ok_or("Cannot serialize current window layout")?;
    crate::atomic_file::write(&positions(mode), text.as_bytes()).map_err(|e| e.to_string())?;
    let data = serde_json::to_vec_pretty(&FocusPanels::capture(panels)).map_err(|e| e.to_string())?;
    crate::atomic_file::write(&windows(mode), &data).map_err(|e| e.to_string())
}

/// Save the outgoing workspace, restore the incoming one's positions and
/// relevant panels. The first visit gets useful default open windows; the
/// second visit restores the user's own arrangement.
pub fn switch(ctx: &egui::Context, from: Mode, to: Mode, panels: &mut Panels) -> Result<(), String> {
    if from == to {
        return Ok(());
    }
    capture(ctx, from, panels)?;
    match std::fs::read_to_string(positions(to)) {
        Ok(text) => layout::apply(ctx, &text)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.to_string()),
    };
    let focus = match std::fs::read(windows(to)) {
        Ok(bytes) => serde_json::from_slice::<FocusPanels>(&bytes).map_err(|e| e.to_string())?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => FocusPanels::defaults(to),
        Err(error) => return Err(error.to_string()),
    };
    focus.apply(panels);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_names_are_distinct_and_file_safe() {
        assert_eq!(Mode::ALL.map(Mode::slug), ["everyday", "studio", "creator"]);
        assert_eq!(Mode::default(), Mode::Everyday);
    }

    #[test]
    fn focused_windows_have_different_roles() {
        let everyday = FocusPanels::defaults(Mode::Everyday);
        let studio = FocusPanels::defaults(Mode::Studio);
        let creator = FocusPanels::defaults(Mode::Creator);
        assert!(everyday.chat && everyday.minimap);
        assert!(studio.snapshot && studio.camera_controls && studio.animation_explorer);
        assert!(creator.about_land && creator.area_search);
        assert!(!everyday.snapshot && !studio.about_land && !creator.camera_controls);
    }

    #[test]
    fn saved_focused_windows_round_trip() {
        let original = FocusPanels::defaults(Mode::Studio);
        let json = serde_json::to_string(&original).unwrap();
        let copy: FocusPanels = serde_json::from_str(&json).unwrap();
        assert_eq!(copy.snapshot, original.snapshot);
        assert_eq!(copy.animation_explorer, original.animation_explorer);
    }
}
