//! Saturn desktop UI, using a Material-inspired navigation and surface system.

pub mod theme;
mod accounts;
mod downloads;
mod home;
mod runtime;
mod settings;
mod versions;

use eframe::egui::{self, Color32};

use crate::core::account::LocalAccountStore;
use crate::core::versions::ManifestController;

pub use accounts::AccountsPageState;
pub use versions::VersionsPageState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Home,
    Versions,
    Downloads,
    Accounts,
    Runtime,
    Settings,
}

impl Page {
    const ALL: [Self; 6] = [
        Self::Home,
        Self::Versions,
        Self::Downloads,
        Self::Accounts,
        Self::Runtime,
        Self::Settings,
    ];

    const fn title(self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Versions => "Versions",
            Self::Downloads => "Downloads",
            Self::Accounts => "Accounts",
            Self::Runtime => "Runtime",
            Self::Settings => "Settings",
        }
    }

    const fn subtitle(self) -> &'static str {
        match self {
            Self::Home => "Your space for Minecraft Java",
            Self::Versions => "Browse versions by release family",
            Self::Downloads => "Track game and runtime downloads",
            Self::Accounts => "Manage local player identities",
            Self::Runtime => "Manage Java runtime environments",
            Self::Settings => "Customize Saturn and its storage",
        }
    }

    const fn symbol(self) -> &'static str {
        match self {
            Self::Home => "⌂",
            Self::Versions => "▦",
            Self::Downloads => "↓",
            Self::Accounts => "◉",
            Self::Runtime => "◌",
            Self::Settings => "⚙",
        }
    }
}

pub fn show_sidebar(ui: &mut egui::Ui, current_page: &mut Page) {
    ui.add_space(5.0);
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(52.0, 52.0), egui::Sense::hover());
        theme::paint_saturn(ui.painter(), rect.center(), 12.0);
        ui.vertical(|ui| {
            ui.add_space(4.0);
            ui.label(egui::RichText::new("Saturn").size(21.0).strong());
            ui.label(egui::RichText::new("MINECRAFT JAVA").size(9.0).strong().color(theme::MUTED));
        });
    });
    ui.add_space(30.0);
    ui.label(egui::RichText::new("LIBRARY").size(10.0).strong().color(theme::MUTED));
    ui.add_space(10.0);

    for page in Page::ALL {
        let selected = *current_page == page;
        let (fill, foreground) = if selected {
            (Color32::from_rgb(91, 65, 34), theme::PRIMARY)
        } else {
            (Color32::TRANSPARENT, theme::MUTED)
        };
        let label = format!("{}     {}", page.symbol(), page.title());
        let button = egui::Button::new(egui::RichText::new(label).size(14.0).color(foreground))
            .fill(fill)
            .stroke(egui::Stroke::NONE)
            .corner_radius(egui::CornerRadius::same(20));
        if ui.add_sized([ui.available_width(), 46.0], button).clicked() {
            *current_page = page;
        }
        ui.add_space(4.0);
    }

    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
        ui.separator();
        ui.label(egui::RichText::new("PHASE 2 · VERSION METADATA").size(9.0).strong().color(theme::MUTED));
        ui.label(egui::RichText::new("Windows x64").size(10.0).color(theme::MUTED));
        ui.add_space(4.0);
    });
}

pub fn show_top_bar(ui: &mut egui::Ui, current_page: Page, accounts: Option<&LocalAccountStore>) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(current_page.title()).size(22.0).strong());
            ui.label(egui::RichText::new(current_page.subtitle()).size(12.0).color(theme::MUTED));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let active_name = accounts
                .and_then(LocalAccountStore::selected_profile)
                .map(|profile| profile.username.as_str())
                .unwrap_or("No profile");
            egui::Frame::new()
                .fill(theme::SURFACE_CONTAINER)
                .stroke(egui::Stroke::new(1.0, theme::OUTLINE))
                .corner_radius(egui::CornerRadius::same(20))
                .inner_margin(egui::Margin::symmetric(12, 7))
                .show(ui, |ui| {
                    ui.label(egui::RichText::new(format!("◉  {active_name}")).size(11.0).color(theme::SECONDARY));
                });
        });
    });
}

#[allow(clippy::too_many_arguments)]
pub fn show_page(
    ui: &mut egui::Ui,
    page: Page,
    data_directory: &str,
    data_directory_ready: bool,
    startup_message: &str,
    accounts: Option<&mut LocalAccountStore>,
    account_error: Option<&str>,
    accounts_page: &mut AccountsPageState,
    manifest_controller: Option<&ManifestController>,
    versions_page: &mut VersionsPageState,
) {
    match page {
        Page::Home => home::show(
            ui,
            data_directory_ready,
            startup_message,
            accounts.as_deref(),
        ),
        Page::Versions => match manifest_controller {
            Some(controller) => versions::show(ui, controller, versions_page),
            None => notice(ui, "Manifest unavailable", "Saturn could not initialize the metadata cache."),
        },
        Page::Downloads => downloads::show(ui),
        Page::Accounts => match accounts {
            Some(store) => accounts::show(ui, store, accounts_page),
            None => {
                section_heading(ui, "Local profiles unavailable", "Saturn could not open its local profile database.");
                notice(ui, "Storage needs attention", account_error.unwrap_or("The data directory is unavailable."));
            }
        },
        Page::Runtime => runtime::show(ui),
        Page::Settings => settings::show(ui, data_directory, data_directory_ready),
    }
}

pub(super) fn section_heading(ui: &mut egui::Ui, title: &str, description: &str) {
    ui.label(egui::RichText::new(title).size(20.0).strong());
    ui.add_space(4.0);
    ui.label(egui::RichText::new(description).size(12.0).color(theme::MUTED));
    ui.add_space(16.0);
}

pub(super) fn info_card(ui: &mut egui::Ui, title: &str, value: &str, detail: &str) {
    theme::card(ui, |ui| {
        ui.set_min_width(170.0);
        ui.label(egui::RichText::new(title).size(10.0).strong().color(theme::MUTED));
        ui.add_space(5.0);
        ui.label(egui::RichText::new(value).size(21.0).strong());
        ui.add_space(3.0);
        ui.label(egui::RichText::new(detail).size(10.0).color(theme::MUTED));
    });
}

pub(super) fn notice(ui: &mut egui::Ui, title: &str, body: &str) {
    egui::Frame::new()
        .fill(Color32::from_rgb(67, 53, 35))
        .stroke(egui::Stroke::new(1.0, Color32::from_rgb(128, 96, 54)))
        .corner_radius(egui::CornerRadius::same(18))
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(title).strong().color(theme::PRIMARY));
            ui.add_space(4.0);
            ui.label(egui::RichText::new(body).size(12.0).color(theme::TEXT));
        });
}

