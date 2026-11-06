#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod app;
mod core;
mod storage;
mod ui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("RustLauncher")
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([960.0, 640.0]),
        ..Default::default()
    };

    eframe::run_native(
        "RustLauncher",
        options,
        Box::new(|creation_context| Ok(Box::new(app::RustLauncherApp::new(creation_context)))),
    )
}
