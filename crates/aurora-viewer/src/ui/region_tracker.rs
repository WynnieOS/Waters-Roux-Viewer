//! Local region tracker backed by real teleport/session history.
//! Only displays places the viewer actually visited; not remote region pings.
//! A hidden-region RLVa setting is respected through the already redacted
//! location entries in the world history model.

use super::widgets::Floater;
use crate::theme::Palette;
use crate::world::tphistory::Session;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    OpenHistory,
    OpenMap,
}

/// One entry per distinct region, most recently visited first.
pub fn recent_regions(session: Session<'_>) -> Vec<(String, usize)> {
    let mut result = Vec::<(String, usize)>::new();
    for entry in session.items.iter().rev() {
        if entry.region.is_empty() {
            continue;
        }
        if let Some((_, visits)) = result.iter_mut().find(|(name, _)| *name == entry.region) {
            *visits += 1;
        } else {
            result.push((entry.region.clone(), 1));
        }
    }
    result
}

pub fn show(ctx: &egui::Context, p: &Palette, open: &mut bool, session: Session<'_>) -> Option<Action> {
    let mut action = None;
    Floater::new(
        "region_tracker",
        "Region Tracker",
        egui::pos2(480.0, 125.0),
        egui::vec2(400.0, 440.0),
    )
    .show(ctx, p, open, |ui| {
        ui.label("Regions visited in this session");
        ui.add_space(6.0);
        let current = session.items.get(session.current);
        if let Some(current) = current {
            ui.label(format!("Current: {}", current.title()));
        } else {
            ui.label("Current region information is not available yet.");
        }
        ui.horizontal(|ui| {
            if ui.button("Open history").clicked() {
                action = Some(Action::OpenHistory);
            }
            if ui.button("World map").clicked() {
                action = Some(Action::OpenMap);
            }
        });
        ui.add_space(8.0);
        let list = recent_regions(session);
        ui.label(format!("Visited regions: {}", list.len()));
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (name, visits) in list {
                ui.horizontal(|ui| {
                    ui.label(name);
                    if visits > 1 {
                        ui.small(format!("{visits} visits"));
                    }
                });
            }
        });
    });
    action
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::tphistory::TpEntry;

    #[test]
    fn region_visits_are_distinct_and_sorted_by_recent_arrival() {
        let create = |region: &str| TpEntry {
            region: region.into(),
            handle: 0,
            position: glam::Vec3::ZERO,
            parcel: String::new(),
            at: 0,
            grid: None,
        };
        let entries = [create("A"), create("B"), create("A")];
        assert_eq!(
            recent_regions(Session {
                items: &entries,
                current: 2
            }),
            vec![("A".into(), 2), ("B".into(), 1)]
        );
    }
}
