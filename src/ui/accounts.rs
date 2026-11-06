use eframe::egui;

use super::{notice, section_heading};

pub fn show(ui: &mut egui::Ui) {
    section_heading(
        ui,
        "Local accounts",
        "Manage offline player profiles without Microsoft sign-in.",
    );
    notice(
        ui,
        "Local profile support is planned for Phase 1",
        "Profiles will have a username and a stable offline UUID. Local mode does not authenticate players and cannot join servers that require authenticated online-mode accounts.",
    );
    ui.add_space(12.0);
    ui.label(egui::RichText::new("Planned features").strong());
    for feature in [
        "Create and edit multiple local profiles",
        "Remember the last selected profile",
        "Generate a consistent offline UUID",
        "Keep account data separate from game instances",
    ] {
        ui.label(format!("•  {feature}"));
    }
}
