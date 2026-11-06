#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod app;
mod core;
mod storage;
mod ui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Saturn Launcher")
            .with_inner_size([1240.0, 800.0])
            .with_min_inner_size([980.0, 680.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Saturn Launcher",
        options,
        Box::new(|creation_context| Ok(Box::new(app::SaturnApp::new(creation_context)))),
    )
}
