use eframe::egui;

use super::{notice, section_heading, theme};

pub fn show(ui: &mut egui::Ui) {
    section_heading(
        ui,
        "Download manager",
        "A single place for game, asset, library and runtime transfer tasks.",
    );
    notice(
        ui,
        "No active downloads",
        "Queue management, parallel transfers, progress reporting, retry, resume and checksum verification arrive in Phase 3.",
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
                    .color(theme::MUTED),
            );
            ui.label("·");
        }
    });
}
