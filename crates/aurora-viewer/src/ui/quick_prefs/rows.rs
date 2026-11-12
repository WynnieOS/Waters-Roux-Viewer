//! Waters Roux: saved, user-configurable Quick Preferences rows.
//! This is local UI state only; the setting sliders still edit their
//! existing Aurora Settings fields, without introducing duplicate settings.

use super::Slot;
use std::path::PathBuf;

fn path() -> PathBuf {
    crate::settings::config_dir().join("quickprefs_rows.json")
}

/// Keep only recognized, unique, customizable rows in the user's order.
/// A saved empty list is intentional: all optional sliders were hidden.
fn normalize(rows: impl IntoIterator<Item = Slot>) -> Vec<Slot> {
    let mut result = Vec::new();
    for slot in rows {
        if Slot::ROWS.contains(&slot) && !result.contains(&slot) {
            result.push(slot);
        }
    }
    result
}

fn load() -> Vec<Slot> {
    let Ok(bytes) = std::fs::read(path()) else {
        return Slot::ROWS.to_vec();
    };
    serde_json::from_slice::<Vec<Slot>>(&bytes).map(normalize).unwrap_or_else(|err| {
        log::warn!("quick preferences row layout ignored: {err}");
        Slot::ROWS.to_vec()
    })
}

fn save(visible: &[Slot]) {
    let path = path();
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        if let Some(folder) = path.parent() {
            std::fs::create_dir_all(folder)?;
        }
        crate::atomic_file::write(&path, &serde_json::to_vec_pretty(visible)?)?;
        Ok(())
    })();
    if let Err(err) = result {
        log::warn!("quick preferences row layout not saved: {err}");
    }
}

/// The optional setting rows and their inline editor. Hover Height,
/// Bandwidth, Max Complexity, Name Tags and the environment remain fixed
/// so the user cannot accidentally remove important controls.
pub struct Editor {
    pub visible: Vec<Slot>,
    editing: bool,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            visible: load(),
            editing: false,
        }
    }
}

impl Editor {
    pub fn show(&mut self, ui: &mut egui::Ui, p: &crate::theme::Palette) {
        ui.horizontal(|ui| {
            let label = if self.editing {
                aurora_i18n::tr!("common-close")
            } else {
                aurora_i18n::tr!("common-edit")
            };
            if ui.small_button(label).clicked() {
                self.editing = !self.editing;
            }
            if self.editing && ui.small_button(aurora_i18n::tr!("common-reset")).clicked() {
                self.visible = Slot::ROWS.to_vec();
                save(&self.visible);
            }
        });
        if !self.editing {
            return;
        }
        crate::ui::widgets::divider(ui, p);
        for slot in Slot::ROWS {
            let mut checked = self.visible.contains(&slot);
            ui.horizontal(|ui| {
                if crate::ui::widgets::checkbox(ui, p, &mut checked, slot.label()).changed() {
                    if checked {
                        self.visible.push(slot);
                    } else {
                        self.visible.retain(|row| *row != slot);
                    }
                    save(&self.visible);
                }
                if let Some(index) = self.visible.iter().position(|row| *row == slot) {
                    let up = ui
                        .add_enabled(index > 0, egui::Button::new("↑"))
                        .on_hover_text(aurora_i18n::tr!("ui-appearance-move-layer-up"));
                    if up.clicked() {
                        self.visible.swap(index, index - 1);
                        save(&self.visible);
                    }
                    let down = ui
                        .add_enabled(index + 1 < self.visible.len(), egui::Button::new("↓"))
                        .on_hover_text(aurora_i18n::tr!("ui-appearance-move-layer-down"));
                    if down.clicked() {
                        self.visible.swap(index, index + 1);
                        save(&self.visible);
                    }
                }
            });
        }
        crate::ui::widgets::divider(ui, p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_layout_deduplicates_and_keeps_user_order() {
        assert_eq!(
            normalize([Slot::Lod, Slot::Particles, Slot::Lod, Slot::Bandwidth]),
            vec![Slot::Lod, Slot::Particles]
        );
    }

    #[test]
    fn hiding_every_optional_row_is_valid() {
        assert!(normalize([]).is_empty());
    }

    #[test]
    fn row_layout_serializes_and_restores() {
        let rows = vec![Slot::Avatars, Slot::Lod];
        let json = serde_json::to_vec(&rows).expect("serialize");
        let decoded: Vec<Slot> = serde_json::from_slice(&json).expect("deserialize");
        assert_eq!(normalize(decoded), rows);
    }
}
