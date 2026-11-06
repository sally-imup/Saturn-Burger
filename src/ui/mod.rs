//! Desktop UI shell and page navigation.

mod accounts;
mod downloads;
mod home;
mod runtime;
mod settings;
mod versions;

use eframe::egui;

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
            Self::Home => "Your launcher overview",
            Self::Versions => "Browse separately organized game versions",
            Self::Downloads => "Track installation and update tasks",
            Self::Accounts => "Manage local player profiles",
            Self::Runtime => "Manage Java runtimes",
            Self::Settings => "Application preferences and data locations",
        }
    }
}

pub fn show_sidebar(ui: &mut egui::Ui, current_page: &mut Page) {
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("R")
                .size(28.0)
                .strong()
                .color(egui::Color32::from_rgb(115, 176, 255)),
        );
        ui.vertical(|ui| {
            ui.label(egui::RichText::new("RustLauncher").size(17.0).strong());
            ui.label(
                egui::RichText::new("MINECRAFT JAVA")
                    .size(9.0)
                    .color(egui::Color32::from_gray(145)),
            );
        });
    });
    ui.add_space(30.0);
    ui.label(
        egui::RichText::new("WORKSPACE")
            .size(10.0)
            .strong()
            .color(egui::Color32::from_gray(130)),
    );
    ui.add_space(8.0);

    for page in Page::ALL {
        let selected = *current_page == page;
        let text_color = if selected {
            egui::Color32::from_rgb(225, 238, 255)
        } else {
            egui::Color32::from_rgb(175, 186, 202)
        };
        let background = if selected {
            egui::Color32::from_rgb(40, 63, 91)
        } else {
            egui::Color32::TRANSPARENT
        };
        let button = egui::Button::new(
            egui::RichText::new(page.title())
                .size(14.0)
                .color(text_color),
        )
        .selected(selected)
        .fill(background)
        .stroke(egui::Stroke::NONE)
        .corner_radius(8);

        if ui
            .add_sized([ui.available_width(), 40.0], button)
            .clicked()
        {
            *current_page = page;
        }
        ui.add_space(3.0);
    }

    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
        ui.separator();
        ui.label(
            egui::RichText::new("Phase 0 · Foundation")
                .size(11.0)
                .color(egui::Color32::from_gray(135)),
        );
        ui.label(
            egui::RichText::new("Windows x64")
                .size(11.0)
                .color(egui::Color32::from_gray(110)),
        );
    });
}

pub fn show_top_bar(ui: &mut egui::Ui, current_page: Page) {
    ui.add_space(13.0);
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(current_page.title()).size(21.0).strong());
            ui.label(
                egui::RichText::new(current_page.subtitle())
                    .size(12.0)
                    .color(egui::Color32::from_gray(145)),
            );
        });
        ui.with_layout(
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                ui.label(
                    egui::RichText::new("LOCAL MODE · FOUNDATION BUILD")
                        .size(10.0)
                        .strong()
                        .color(egui::Color32::from_rgb(151, 194, 255)),
                );
            },
        );
    });
    ui.add_space(13.0);
}

pub fn show_page(
    ui: &mut egui::Ui,
    page: Page,
    data_directory: &str,
    data_directory_ready: bool,
    startup_message: &str,
) {
    match page {
        Page::Home => home::show(ui, data_directory_ready, startup_message),
        Page::Versions => versions::show(ui),
        Page::Downloads => downloads::show(ui),
        Page::Accounts => accounts::show(ui),
        Page::Runtime => runtime::show(ui),
        Page::Settings => settings::show(ui, data_directory, data_directory_ready),
    }
}

pub(super) fn section_heading(ui: &mut egui::Ui, title: &str, description: &str) {
    ui.label(egui::RichText::new(title).size(17.0).strong());
    ui.add_space(3.0);
    ui.label(
        egui::RichText::new(description)
            .size(12.0)
            .color(egui::Color32::from_gray(145)),
    );
    ui.add_space(14.0);
}

pub(super) fn info_card(ui: &mut egui::Ui, title: &str, value: &str, detail: &str) {
    egui::Frame::group(ui.style())
        .fill(egui::Color32::from_rgb(25, 32, 44))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(43, 53, 70)))
        .corner_radius(12)
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.set_min_width(170.0);
            ui.label(
                egui::RichText::new(title)
                    .size(11.0)
                    .color(egui::Color32::from_gray(150)),
            );
            ui.add_space(6.0);
            ui.label(egui::RichText::new(value).size(20.0).strong());
            ui.add_space(3.0);
            ui.label(
                egui::RichText::new(detail)
                    .size(11.0)
                    .color(egui::Color32::from_gray(135)),
            );
        });
}

pub(super) fn notice(ui: &mut egui::Ui, title: &str, body: &str) {
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(31, 43, 59))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(55, 78, 105)))
        .corner_radius(10)
        .inner_margin(egui::Margin::same(14))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(title)
                    .strong()
                    .color(egui::Color32::from_rgb(177, 211, 255)),
            );
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(body)
                    .size(12.0)
                    .color(egui::Color32::from_gray(195)),
            );
        });
}
