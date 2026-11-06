use eframe::egui;

use crate::core::versions::VersionCategory;

use super::{notice, section_heading};

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
                egui::Frame::group(ui.style())
                    .fill(egui::Color32::from_rgb(25, 32, 44))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(43, 53, 70)))
                    .corner_radius(10)
                    .inner_margin(egui::Margin::same(14))
                    .show(ui, |ui| {
                        ui.set_min_width(300.0);
                        ui.label(egui::RichText::new(category.label()).size(14.0).strong());
                        ui.label(
                            egui::RichText::new(description)
                                .size(11.0)
                                .color(egui::Color32::from_gray(145)),
                        );
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new(directory)
                                .monospace()
                                .size(10.0)
                                .color(egui::Color32::from_gray(125)),
                        );
                        ui.label(
                            egui::RichText::new("Not connected yet")
                                .size(10.0)
                                .color(egui::Color32::from_rgb(225, 183, 112)),
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
        "The category layout is prepared, but no Mojang manifest is fetched and no version is installable in this Phase 0 build.",
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
