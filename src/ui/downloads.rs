use eframe::egui;

use super::{notice, section_heading};

pub fn show(ui: &mut egui::Ui) {
    section_heading(
        ui,
        "Download manager",
        "A single place for installation, runtime and asset download tasks.",
    );
    notice(
        ui,
        "No active downloads",
        "Queue management, parallel transfers, progress reporting, retry, resume and checksum verification will be implemented in Phase 3.",
    );
    ui.add_space(14.0);
    ui.label(egui::RichText::new("Planned download states").strong());
    ui.add_space(8.0);
    ui.horizontal_wrapped(|ui| {
        for state in [
            "Queued",
            "Downloading",
            "Verifying",
            "Retrying",
            "Completed",
            "Failed",
            "Cancelled",
        ] {
            ui.label(
                egui::RichText::new(state)
                    .size(11.0)
                    .color(egui::Color32::from_gray(175)),
            );
            ui.label("·");
        }
    });
}
