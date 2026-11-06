use eframe::egui;

use crate::core::versions::VersionCategory;

use super::{notice, section_heading, theme};

pub fn show(ui: &mut egui::Ui) {
    section_heading(
        ui,
        "Version browser",
        "Version groups are designed to stay separate instead of mixing every release together.",
    );

    egui::Grid::new("version_category_grid")
        .num_columns(2)
        .spacing([12.0, 12.0])
        .show(ui, |ui| {
            for (index, category) in VersionCategory::ALL.into_iter().enumerate() {
                let (description, directory) = category_details(category);
                theme::card(ui, |ui| {
                    ui.set_min_width(300.0);
                    ui.label(egui::RichText::new(category.label()).size(14.0).strong());
                    ui.label(
                        egui::RichText::new(description)
                            .size(11.0)
                            .color(theme::MUTED),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(directory)
                            .monospace()
                            .size(10.0)
                            .color(theme::MUTED),
                    );
                    ui.label(
                        egui::RichText::new("Not connected yet")
                            .size(10.0)
                            .color(theme::PRIMARY),
                    );
                });
                if index % 2 == 1 {
                    ui.end_row();
                }
            }
        });

    ui.add_space(16.0);
    notice(
        ui,
        "Metadata integration is Phase 2",
        "The category layout is prepared, but the Mojang manifest and installer are implemented in Phases 2 and 3.",
    );
}

fn category_details(category: VersionCategory) -> (&'static str, &'static str) {
    match category {
        VersionCategory::Release => ("Stable releases", "instances/vanilla"),
        VersionCategory::Snapshot => ("Development snapshots", "instances/snapshot"),
        VersionCategory::AprilFools => (
            "Special April Fools versions",
            "instances/april-fools",
        ),
        VersionCategory::PreRelease => (
            "Pre-releases and release candidates",
            "instances/pre-release",
        ),
        VersionCategory::OldBeta => ("Historical beta versions", "instances/old-beta"),
        VersionCategory::OldAlpha => ("Historical alpha versions", "instances/old-alpha"),
        VersionCategory::Experimental => (
            "Other detected or user-provided versions",
            "instances/experimental",
        ),
    }
}
