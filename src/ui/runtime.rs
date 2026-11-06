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
        "Custom JVM is not part of Phase 0",
        "The planned runtime pipeline will detect compatible Java versions, download managed runtimes, validate required modules and optionally create minimized JRE images. It will not replace the JVM with a new implementation.",
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
