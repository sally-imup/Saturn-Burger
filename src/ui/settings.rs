use eframe::egui;

use super::{notice, section_heading, theme};

pub fn show(ui: &mut egui::Ui, data_directory: &str, data_directory_ready: bool) {
    section_heading(
        ui,
        "Settings",
        "Basic application information and storage location.",
    );

    theme::card(ui, |ui| {
        ui.label(egui::RichText::new("Saturn data directory").strong());
        ui.add_space(5.0);
        ui.label(egui::RichText::new(data_directory).monospace().size(11.0));
        ui.add_space(7.0);

        let status = if data_directory_ready {
            "Directory layout created"
        } else {
            "Directory layout not available"
        };
        let status_color = if data_directory_ready {
            egui::Color32::from_rgb(130, 205, 155)
        } else {
            egui::Color32::from_rgb(235, 170, 120)
        };
        ui.label(egui::RichText::new(status).size(11.0).color(status_color));
    });

    ui.add_space(14.0);
    notice(
        ui,
        "More settings will be added incrementally",
        "Theme customization, download concurrency, Java memory presets, game resolution and per-instance options will be added in later phases.",
    );
}
