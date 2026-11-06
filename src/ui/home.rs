use eframe::egui;

use crate::core::account::LocalAccountStore;

use super::{info_card, notice, section_heading, theme};

pub fn show(
    ui: &mut egui::Ui,
    data_directory_ready: bool,
    startup_message: &str,
    accounts: Option<&LocalAccountStore>,
) {
    section_heading(
        ui,
        "A launcher with a planetary perspective",
        "Your Minecraft Java library, player profiles, and runtime tools in one place.",
    );

    ui.horizontal_wrapped(|ui| {
        let profiles = match accounts {
            Some(store) => store.profiles().len(),
            None => 0,
        };
        let active = accounts
            .and_then(LocalAccountStore::selected_profile)
            .map_or("Not selected", |profile| profile.username.as_str());
        info_card(ui, "PROJECT", "Saturn", "Windows x64 launcher");
        info_card(ui, "LOCAL PROFILES", &profiles.to_string(), "Offline identities");
        info_card(ui, "ACTIVE PLAYER", active, "Current local profile");
        info_card(ui, "JAVA RUNTIME", "Coming later", "Custom JRE in Phase 4");
    });

    ui.add_space(18.0);
    let storage_title = if data_directory_ready {
        "Saturn storage is ready"
    } else {
        "Storage setup needs attention"
    };
    notice(ui, storage_title, startup_message);

    ui.add_space(18.0);
    section_heading(ui, "Development roadmap", "Each milestone adds a tested launcher subsystem.");
    theme::card(ui, |ui| {
        for (phase, title, status, active) in [
            ("00", "Foundation and desktop shell", "DONE", false),
            ("01", "Local profile management", "CURRENT", true),
            ("02", "Version manifest and classifier", "NEXT", false),
            ("03", "Downloader and isolated installer", "PLANNED", false),
            ("04", "Java runtime manager and custom JRE", "PLANNED", false),
            ("05", "Minecraft launch engine", "PLANNED", false),
        ] {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(phase).monospace().strong().color(theme::PRIMARY));
                ui.label(title);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(status)
                            .size(9.0)
                            .strong()
                            .color(if active { theme::PRIMARY } else { theme::MUTED }),
                    );
                });
            });
            ui.add_space(8.0);
        }
    });
}
