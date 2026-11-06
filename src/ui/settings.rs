use eframe::egui;

use super::{notice, section_heading};

pub fn show(ui: &mut egui::Ui, data_directory: &str, data_directory_ready: bool) {
    section_heading(
        ui,
        "Settings",
        "Basic application information and storage location.",
    );

    egui::Frame::group(ui.style())
        .fill(egui::Color32::from_rgb(25, 32, 44))
        .corner_radius(10)
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.label(egui::RichText::new("Application data directory").strong());
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
        "Theme customization, download concurrency, Java memory presets, game resolution and per-instance options are not implemented in Phase 0.",
    );
}
