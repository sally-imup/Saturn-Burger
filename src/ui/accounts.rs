use eframe::egui::{self, Color32};

use crate::core::account::LocalAccountStore;

use super::theme;

#[derive(Default)]
pub struct AccountsPageState {
    pub new_username: String,
    pub rename_username: String,
    pub editing_uuid: Option<String>,
    pub confirm_delete: bool,
    pub feedback: Option<(bool, String)>,
}

impl AccountsPageState {
    fn set_feedback(&mut self, result: Result<(), String>, success_message: &str) {
        self.feedback = Some(match result {
            Ok(()) => (true, success_message.to_owned()),
            Err(error) => (false, error),
        });
    }
}

pub fn show(ui: &mut egui::Ui, store: &mut LocalAccountStore, state: &mut AccountsPageState) {
    ui.label(egui::RichText::new("Local profiles").size(25.0).strong());
    ui.label(
        egui::RichText::new("Create and manage offline player identities for local play.")
            .size(13.0)
            .color(theme::MUTED),
    );
    ui.add_space(18.0);

    if let Some(selected) = store.selected_profile() {
        if state.editing_uuid.as_deref() != Some(selected.offline_uuid.as_str()) {
            state.editing_uuid = Some(selected.offline_uuid.clone());
            state.rename_username = selected.username.clone();
            state.confirm_delete = false;
        }
    } else {
        state.editing_uuid = None;
        state.rename_username.clear();
        state.confirm_delete = false;
    }

    egui::Grid::new("accounts_overview_grid")
        .num_columns(2)
        .spacing([14.0, 0.0])
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.set_min_width(270.0);
                theme::card(ui, |ui| {
                    ui.label(egui::RichText::new("CREATE PROFILE").size(11.0).strong().color(theme::SECONDARY));
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("Player name").size(12.0).color(theme::MUTED));
                    ui.add(
                        egui::TextEdit::singleline(&mut state.new_username)
                            .hint_text("e.g. SaturnPlayer")
                            .desired_width(f32::INFINITY),
                    );
                    ui.add_space(8.0);
                    if theme::primary_button(ui, "＋  Create local profile").clicked() {
                        let result = store.create_profile(&state.new_username);
                        if result.is_ok() {
                            state.new_username.clear();
                            state.confirm_delete = false;
                        }
                        state.set_feedback(result, "Local profile created and selected.");
                    }
                    ui.add_space(5.0);
                    ui.label(
                        egui::RichText::new("3–16 characters · letters, numbers, underscore")
                            .size(10.0)
                            .color(theme::MUTED),
                    );
                });

                ui.add_space(12.0);
                theme::card(ui, |ui| {
                    ui.label(egui::RichText::new("LOCAL MODE").size(11.0).strong().color(theme::SECONDARY));
                    ui.add_space(7.0);
                    ui.label(
                        egui::RichText::new("No Microsoft sign-in is used. Offline profiles do not authenticate with Mojang and cannot join servers that require authenticated online-mode accounts.")
                            .size(12.0)
                            .color(theme::MUTED),
                    );
                });
            });
            ui.vertical(|ui| {
                ui.set_min_width(380.0);
                theme::card(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("YOUR PROFILES").size(11.0).strong().color(theme::SECONDARY));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new(format!("{} total", store.profiles().len())).size(11.0).color(theme::MUTED));
                        });
                    });
                    ui.add_space(8.0);

                    if store.profiles().is_empty() {
                        ui.add_space(12.0);
                        ui.label(egui::RichText::new("No local profiles yet").strong());
                        ui.label(egui::RichText::new("Create your first profile using the form on the left.").size(12.0).color(theme::MUTED));
                        ui.add_space(12.0);
                    } else {
                        let profiles = store
                            .profiles()
                            .iter()
                            .map(|profile| (profile.username.clone(), profile.offline_uuid.clone()))
                            .collect::<Vec<_>>();
                        let selected_uuid = store.selected_uuid().map(str::to_owned);
                        let mut requested_selection = None;

                        for (username, offline_uuid) in profiles {
                            let selected = selected_uuid.as_deref() == Some(offline_uuid.as_str());
                            let fill = if selected { theme::SURFACE_HIGHEST } else { theme::SURFACE };
                            egui::Frame::new()
                                .fill(fill)
                                .stroke(egui::Stroke::new(1.0, if selected { theme::PRIMARY } else { theme::OUTLINE }))
                                .corner_radius(egui::CornerRadius::same(16))
                                .inner_margin(egui::Margin::same(10))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        let (avatar_rect, _) = ui.allocate_exact_size(egui::vec2(38.0, 38.0), egui::Sense::hover());
                                        ui.painter().circle_filled(avatar_rect.center(), 19.0, Color32::from_rgb(102, 76, 43));
                                        let initial = username.chars().next().unwrap_or('?').to_ascii_uppercase().to_string();
                                        ui.painter().text(avatar_rect.center(), egui::Align2::CENTER_CENTER, initial, egui::FontId::proportional(17.0), theme::PRIMARY);
                                        ui.vertical(|ui| {
                                            ui.label(egui::RichText::new(username.as_str()).strong());
                                            ui.label(egui::RichText::new(if selected { "Currently selected" } else { "Local profile" }).size(10.0).color(theme::MUTED));
                                        });
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            if !selected && theme::secondary_button(ui, "Select").clicked() {
                                                requested_selection = Some(offline_uuid.clone());
                                            }
                                        });
                                    });
                                });
                            ui.add_space(6.0);
                        }

                        if let Some(uuid) = requested_selection {
                            state.set_feedback(store.select_profile(&uuid), "Profile selected.");
                            state.confirm_delete = false;
                        }
                    }
                });
            });
            ui.end_row();
        });

    ui.add_space(14.0);
    if let Some(profile) = store.selected_profile() {
        let selected_username = profile.username.clone();
        let selected_uuid = profile.offline_uuid.clone();
        theme::card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("SELECTED PROFILE").size(11.0).strong().color(theme::SECONDARY));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new("OFFLINE IDENTITY").size(10.0).color(theme::MUTED));
                });
            });
            ui.add_space(8.0);
            ui.label(egui::RichText::new(selected_username.as_str()).size(21.0).strong());
            ui.label(egui::RichText::new(format!("UUID  {selected_uuid}")).monospace().size(11.0).color(theme::MUTED));
            ui.add_space(14.0);
            ui.separator();
            ui.add_space(8.0);
            ui.label(egui::RichText::new("Rename profile").strong());
            ui.add(
                egui::TextEdit::singleline(&mut state.rename_username)
                    .hint_text("New player name")
                    .desired_width(280.0),
            );
            ui.horizontal_wrapped(|ui| {
                if theme::secondary_button(ui, "Save new name").clicked() {
                    let result = store.rename_selected(&state.rename_username);
                    state.set_feedback(result, "Profile renamed. Its offline UUID has changed too.");
                    if let Some(profile) = store.selected_profile() {
                        state.editing_uuid = Some(profile.offline_uuid.clone());
                        state.rename_username = profile.username.clone();
                    }
                }
                if theme::secondary_button(ui, "Delete profile").clicked() {
                    state.confirm_delete = true;
                }
            });
            if state.confirm_delete {
                ui.add_space(8.0);
                egui::Frame::new()
                    .fill(Color32::from_rgb(73, 39, 32))
                    .corner_radius(egui::CornerRadius::same(12))
                    .inner_margin(egui::Margin::same(12))
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(format!("Delete {selected_username}?")).strong().color(theme::ERROR));
                        ui.label(egui::RichText::new("This removes the launcher profile, not any Minecraft world or instance files.").size(11.0));
                        ui.horizontal(|ui| {
                            if ui.button("Confirm delete").clicked() {
                                let result = store.delete_selected();
                                state.set_feedback(result, "Profile deleted.");
                                state.confirm_delete = false;
                                state.editing_uuid = None;
                            }
                            if ui.button("Cancel").clicked() {
                                state.confirm_delete = false;
                            }
                        });
                    });
            }
        });
    }

    if let Some((success, message)) = &state.feedback {
        ui.add_space(12.0);
        let color = if *success { theme::SUCCESS } else { theme::ERROR };
        ui.label(egui::RichText::new(message.as_str()).size(12.0).color(color));
    }

    ui.add_space(10.0);
    ui.label(
        egui::RichText::new("Note: renaming a local profile changes its offline UUID. Offline servers may treat the renamed player as a different identity.")
            .size(11.0)
            .color(theme::MUTED),
    );
}

