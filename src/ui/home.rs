use eframe::egui;

use super::{info_card, notice, section_heading};

pub fn show(ui: &mut egui::Ui, data_directory_ready: bool, startup_message: &str) {
    section_heading(
        ui,
        "Welcome to RustLauncher",
        "A clean foundation for a modular Minecraft Java launcher.",
    );

    ui.horizontal_wrapped(|ui| {
        info_card(ui, "PROJECT STAGE", "Phase 0", "Application foundation");
        info_card(ui, "INSTALLED VERSIONS", "0", "Installer arrives in Phase 3");
        info_card(
            ui,
            "LOCAL PROFILES",
            "Not configured",
            "Account system arrives in Phase 1",
        );
        info_card(ui, "JAVA RUNTIME", "Not installed", "Runtime manager arrives later");
    });

    ui.add_space(22.0);
    section_heading(
        ui,
        "Getting started",
        "The shell is ready; launcher services will be connected incrementally.",
    );
    let storage_title = if data_directory_ready {
        "Application storage ready"
    } else {
        "Storage setup needs attention"
    };
    notice(ui, storage_title, startup_message);
    ui.add_space(14.0);

    egui::Frame::group(ui.style())
        .fill(egui::Color32::from_rgb(25, 32, 44))
        .inner_margin(egui::Margin::same(16))
        .corner_radius(12)
        .show(ui, |ui| {
            ui.label(egui::RichText::new("Development roadmap").size(14.0).strong());
            ui.add_space(10.0);
            for (phase, title, status) in [
                ("00", "Foundation, desktop UI and CI", "CURRENT"),
                ("01", "Local / offline profiles", "NEXT"),
                ("02", "Version manifest and categories", "PLANNED"),
                ("03", "Downloader and isolated installer", "PLANNED"),
                ("04", "Java runtime manager and custom JRE", "PLANNED"),
                ("05", "Minecraft launch engine", "PLANNED"),
            ] {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(phase)
                            .monospace()
                            .color(egui::Color32::from_rgb(135, 184, 255)),
                    );
                    ui.label(title);
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.label(
                                egui::RichText::new(status)
                                    .size(9.0)
                                    .strong()
                                    .color(egui::Color32::from_gray(145)),
                            );
                        },
                    );
                });
                ui.add_space(7.0);
            }
        });
}
