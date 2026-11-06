use eframe::egui;

use super::{notice, section_heading};

pub fn show(ui: &mut egui::Ui) {
    section_heading(
        ui,
        "Java runtime manager",
        "Runtime selection and custom lightweight JRE work will arrive after the base launcher systems.",
    );
    notice(
        ui,
        "Custom lightweight JRE is planned for Phase 4",
        "Saturn will manage compatible Java distributions and may build minimized JRE images with jlink after compatibility testing. This is runtime optimization, not a new JVM implementation.",
    );
    ui.add_space(14.0);
    ui.label(egui::RichText::new("Planned runtime profiles").strong());
    for profile in [
        "Automatic",
        "Compatibility",
        "Low memory",
        "Balanced",
        "Performance",
        "Custom arguments",
    ] {
        ui.label(format!("•  {profile}"));
    }
}
