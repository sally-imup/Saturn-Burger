use eframe::egui;

use crate::core::versions::{ManifestController, VersionCategory};

use super::{notice, section_heading, theme};

#[derive(Default)]
pub struct VersionsPageState {
    selected_category: Option<VersionCategory>,
    search: String,
}

pub fn show(ui: &mut egui::Ui, controller: &ManifestController, page: &mut VersionsPageState) {
    section_heading(
        ui,
        "Version browser",
        "Browse official Minecraft Java versions. Each family remains separated for isolated installation in Phase 3.",
    );

    let snapshot = controller.snapshot();
    if snapshot.loading {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(150));
    }
    ui.horizontal_wrapped(|ui| {
        if theme::primary_button(ui, if snapshot.loading { "Refreshing…" } else { "Refresh manifest" }).clicked() {
            controller.refresh();
        }
        if let Some(manifest) = &snapshot.manifest {
            ui.label(egui::RichText::new(format!("{} versions", manifest.versions.len())).color(theme::MUTED));
            if let Some(latest) = &manifest.latest.release {
                ui.label(egui::RichText::new(format!("Latest release: {latest}")).color(theme::SECONDARY));
            }
        }
    });
    if let Some(error) = &snapshot.error {
        notice(ui, "Manifest refresh failed", error);
    }
    if snapshot.manifest.is_none() {
        notice(
            ui,
            "No version manifest cached yet",
            "Connect to the internet and select Refresh manifest to download Mojang's official version list. Previously cached metadata remains available offline.",
        );
        return;
    }

    ui.add_space(12.0);
    ui.horizontal(|ui| {
        ui.label("Search");
        ui.add_sized([220.0, 34.0], egui::TextEdit::singleline(&mut page.search).hint_text("Version ID…"));
        if ui.selectable_label(page.selected_category.is_none(), "All").clicked() {
            page.selected_category = None;
        }
    });

    ui.add_space(8.0);
    let manifest = snapshot.manifest.as_ref().expect("manifest checked above");
    egui::ScrollArea::vertical().show(ui, |ui| {
        for category in VersionCategory::ALL {
            if page.selected_category.is_some_and(|selected| selected != category) {
                continue;
            }
            let versions: Vec<_> = manifest.versions.iter()
                .filter(|version| version.category() == category)
                .filter(|version| version.id.to_ascii_lowercase().contains(&page.search.to_ascii_lowercase()))
                .collect();
            if versions.is_empty() { continue; }

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(category.label()).size(16.0).strong());
                ui.label(egui::RichText::new(format!("{}", versions.len())).size(11.0).color(theme::MUTED));
                if ui.small_button("Show only").clicked() {
                    page.selected_category = Some(category);
                }
            });
            ui.add_space(5.0);
            for version in versions {
                theme::card(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(&version.id).strong().size(13.0));
                            ui.label(egui::RichText::new(&version.release_time).size(10.0).color(theme::MUTED));
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new("Available in Phase 3").size(10.0).color(theme::SECONDARY));
                        });
                    });
                });
                ui.add_space(5.0);
            }
            ui.add_space(12.0);
        }
    });
    if let Some(updated) = snapshot.last_updated {
        ui.label(egui::RichText::new(format!("Manifest refreshed: {updated}")).size(10.0).color(theme::MUTED));
    }
}
