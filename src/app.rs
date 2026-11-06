use eframe::egui;

use crate::storage;
use crate::ui::{self, Page};

pub struct RustLauncherApp {
    current_page: Page,
    data_directory: String,
    data_directory_ready: bool,
    startup_message: String,
}

impl RustLauncherApp {
    pub fn new(_creation_context: &eframe::CreationContext<'_>) -> Self {
        let directory_result = storage::ensure_data_layout();
        let (data_directory, data_directory_ready, startup_message) = match directory_result {
            Ok(path) => (
                path.display().to_string(),
                true,
                "Application data directories are ready.".to_owned(),
            ),
            Err(error) => (
                storage::suggested_data_directory()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "Unavailable".to_owned()),
                false,
                format!("Could not prepare application data directories: {error}"),
            ),
        };

        Self {
            current_page: Page::Home,
            data_directory,
            data_directory_ready,
            startup_message,
        }
    }
}

impl eframe::App for RustLauncherApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.ctx().set_visuals(egui::Visuals::dark());
        let available_size = ui.available_size();

        egui::Frame::new()
            .fill(egui::Color32::from_rgb(17, 21, 29))
            .show(ui, |root_ui| {
                root_ui.horizontal(|root_ui| {
                    root_ui.allocate_ui_with_layout(
                        egui::vec2(218.0, available_size.y),
                        egui::Layout::top_down(egui::Align::Min),
                        |sidebar_ui| {
                            egui::Frame::new()
                                .fill(egui::Color32::from_rgb(22, 28, 38))
                                .inner_margin(egui::Margin::same(16))
                                .show(sidebar_ui, |sidebar_content| {
                                    ui::show_sidebar(sidebar_content, &mut self.current_page);
                                });
                        },
                    );

                    root_ui.separator();
                    root_ui.vertical(|content_ui| {
                        content_ui.set_width(content_ui.available_width());
                        ui::show_top_bar(content_ui, self.current_page);
                        content_ui.separator();
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(content_ui, |page_ui| {
                                page_ui.add_space(18.0);
                                ui::show_page(
                                    page_ui,
                                    self.current_page,
                                    &self.data_directory,
                                    self.data_directory_ready,
                                    &self.startup_message,
                                );
                                page_ui.add_space(20.0);
                            });
                    });
                });
            });
    }
}
