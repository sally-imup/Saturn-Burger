use eframe::egui;

use crate::core::account::LocalAccountStore;
use crate::storage;
use crate::ui::{self, AccountsPageState, Page};

struct StartupState {
    data_directory: String,
    data_directory_ready: bool,
    startup_message: String,
    account_store: Option<LocalAccountStore>,
    account_store_error: Option<String>,
}

pub struct SaturnApp {
    current_page: Page,
    data_directory: String,
    data_directory_ready: bool,
    startup_message: String,
    account_store: Option<LocalAccountStore>,
    account_store_error: Option<String>,
    accounts_page: AccountsPageState,
}

impl SaturnApp {
    pub fn new(_creation_context: &eframe::CreationContext<'_>) -> Self {
        let startup = load_startup_state();

        Self {
            current_page: Page::Home,
            data_directory: startup.data_directory,
            data_directory_ready: startup.data_directory_ready,
            startup_message: startup.startup_message,
            account_store: startup.account_store,
            account_store_error: startup.account_store_error,
            accounts_page: AccountsPageState::default(),
        }
    }
}

impl eframe::App for SaturnApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui::theme::apply(ui.ctx());

        egui::Panel::left("saturn_navigation")
            .exact_size(236.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(ui::theme::SURFACE)
                    .inner_margin(egui::Margin::same(16)),
            )
            .show(ui, |sidebar| {
                ui::show_sidebar(sidebar, &mut self.current_page);
            });

        egui::Panel::top("saturn_top_bar")
            .exact_size(82.0)
            .frame(
                egui::Frame::new()
                    .fill(ui::theme::BACKGROUND)
                    .inner_margin(egui::Margin::symmetric(24, 14)),
            )
            .show(ui, |top| {
                ui::show_top_bar(top, self.current_page, self.account_store.as_ref());
            });

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(ui::theme::BACKGROUND)
                    .inner_margin(egui::Margin::symmetric(24, 12)),
            )
            .show(ui, |content| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(content, |page_ui| {
                        ui::show_page(
                            page_ui,
                            self.current_page,
                            &self.data_directory,
                            self.data_directory_ready,
                            &self.startup_message,
                            self.account_store.as_mut(),
                            self.account_store_error.as_deref(),
                            &mut self.accounts_page,
                        );
                    });
            });
    }
}

fn load_startup_state() -> StartupState {
    match storage::ensure_data_layout() {
        Ok(path) => {
            let display_path = path.display().to_string();
            match LocalAccountStore::load(&path) {
                Ok(store) => StartupState {
                    data_directory: display_path,
                    data_directory_ready: true,
                    startup_message: "Saturn storage and local profile database are ready.".to_owned(),
                    account_store: Some(store),
                    account_store_error: None,
                },
                Err(error) => StartupState {
                    data_directory: display_path,
                    data_directory_ready: true,
                    startup_message: "Saturn storage is ready, but the local profile database needs attention.".to_owned(),
                    account_store: None,
                    account_store_error: Some(error),
                },
            }
        }
        Err(error) => StartupState {
            data_directory: storage::suggested_data_directory()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "Unavailable".to_owned()),
            data_directory_ready: false,
            startup_message: format!("Could not prepare Saturn application data directories: {error}"),
            account_store: None,
            account_store_error: Some("Application storage is unavailable.".to_owned()),
        },
    }
}
